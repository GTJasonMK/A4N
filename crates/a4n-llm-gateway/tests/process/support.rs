use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use a4n_capabilities::{CapabilityClient, HttpLlmClient, JsonRequest, LlmClient};
use a4n_llm::provider::ChatRequest;
use a4n_service_manager::{ManagedServiceConfig, ServiceManager, ServiceManagerError};
use a4n_service_protocol::{
    HealthStatus, LLM_CHAT_CAPABILITY_VERSION, LLM_SERVICE_NAME, PROTOCOL_VERSION,
    ServiceRequestContext, ServiceTaskEvent, TraceContext,
};
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::post,
};
use serde_json::{Value, json};
use tokio::{sync::oneshot, task::JoinHandle, time::timeout};

pub(crate) fn unused_local_addr() -> SocketAddr {
    let listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("temporary listener should reserve a loopback port");
    listener
        .local_addr()
        .expect("temporary listener should expose its local address")
}

fn gateway_binary() -> PathBuf {
    let binary = PathBuf::from(env!("CARGO_BIN_EXE_a4n-llm-gateway"));
    assert!(binary.is_file(), "gateway integration binary must exist");
    binary
}

pub(crate) fn gateway_manager(addr: SocketAddr) -> ServiceManager {
    let mut config =
        ManagedServiceConfig::sidecar(LLM_SERVICE_NAME, gateway_binary(), format!("http://{addr}"));
    config
        .env
        .insert("A4N_LLM_GATEWAY_ADDR".to_string(), addr.to_string());
    config.env.insert(
        "A4N_LLM_GATEWAY_LOG".to_string(),
        "a4n_llm_gateway=error".to_string(),
    );
    config.startup_timeout_ms = 10_000;
    ServiceManager::new(vec![config]).expect("gateway sidecar config should be valid")
}

#[derive(Debug, Clone)]
pub(crate) struct FakeProviderRequest {
    pub(crate) authorization: Option<String>,
    pub(crate) body: Value,
}

type FakeProviderObservation = Arc<Mutex<Vec<FakeProviderRequest>>>;

pub(crate) struct FakeProviderServer {
    addr: SocketAddr,
    observation: FakeProviderObservation,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
}

impl FakeProviderServer {
    pub(crate) async fn start() -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("fake provider should bind a loopback port");
        let addr = listener
            .local_addr()
            .expect("fake provider should expose its local address");
        let observation = Arc::new(Mutex::new(Vec::new()));
        let router = Router::new()
            .route("/chat/completions", post(fake_chat_completions))
            .with_state(observation.clone());
        let (shutdown, shutdown_rx) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_rx.await;
                })
                .await
        });

        Self {
            addr,
            observation,
            shutdown,
            task,
        }
    }

    pub(crate) fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    pub(crate) fn requests(&self) -> Vec<FakeProviderRequest> {
        self.observation
            .lock()
            .expect("fake provider observation lock should not be poisoned")
            .clone()
    }

    pub(crate) async fn stop(self) {
        let _ = self.shutdown.send(());
        timeout(Duration::from_secs(5), self.task)
            .await
            .expect("fake provider should stop before timeout")
            .expect("fake provider task should join")
            .expect("fake provider should stop cleanly");
    }
}

async fn fake_chat_completions(
    State(observation): State<FakeProviderObservation>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    observation
        .lock()
        .expect("fake provider observation lock should not be poisoned")
        .push(FakeProviderRequest {
            authorization,
            body: body.clone(),
        });

    if body["model"] == json!("process-error-500") {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            [(header::CONTENT_TYPE, "application/json")],
            r#"{"error":{"message":"token=process-upstream-secret base_url=https://process-provider.invalid/v1"}}"#,
        )
            .into_response();
    }

    if body["stream"].as_bool() == Some(true) {
        return (
            [(header::CONTENT_TYPE, "text/event-stream")],
            concat!(
                "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"pro\"},\"finish_reason\":null}]}\n\n",
                "data: {\"choices\":[{\"index\":1,\"delta\":{\"content\":\"cess\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":4,\"completion_tokens\":2,\"total_tokens\":6}}\n\n",
                "data: [DONE]\n\n"
            ),
        )
            .into_response();
    }

    Json(json!({
        "choices": [{
            "message": { "content": "process chat ok" },
            "finish_reason": "stop"
        }],
        "usage": {
            "prompt_tokens": 3,
            "completion_tokens": 2,
            "total_tokens": 5
        }
    }))
    .into_response()
}

pub(crate) fn gateway_request(
    request_id: &str,
    trace_id: &str,
    provider_base_url: &str,
    request: ChatRequest,
) -> JsonRequest {
    gateway_request_for_model(
        request_id,
        trace_id,
        provider_base_url,
        "process-fake-model",
        request,
    )
}

pub(crate) fn gateway_request_for_model(
    request_id: &str,
    trace_id: &str,
    provider_base_url: &str,
    provider_model: &str,
    request: ChatRequest,
) -> JsonRequest {
    let mut context = ServiceRequestContext::local_user_secret(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: Some("process-integration".to_string()),
        },
    );
    context.project_id = Some(7);
    context.root_project_id = Some(3);
    context
        .with_payload(
            LLM_CHAT_CAPABILITY_VERSION,
            json!({
                "provider": {
                    "base_url": provider_base_url,
                    "model": provider_model,
                    "api_key": "process-provider-secret",
                    "timeout_secs": 5,
                    "request_options_json": null
                },
                "request": request
            }),
        )
        .expect("process integration request context should be valid")
}

pub(crate) async fn collect_stream_events(
    client: &impl LlmClient,
    request: JsonRequest,
) -> Vec<ServiceTaskEvent> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();
    client
        .stream(request, event_tx)
        .await
        .expect("terminal event should complete typed transport");
    let mut events = Vec::new();
    while let Some(event) = event_rx.recv().await {
        events.push(event);
    }
    events
}

pub(crate) async fn wait_until_unavailable(manager: &ServiceManager) -> ServiceManagerError {
    timeout(Duration::from_secs(5), async {
        loop {
            match manager.health(LLM_SERVICE_NAME).await {
                Ok(_) => tokio::task::yield_now().await,
                Err(error) => return error,
            }
        }
    })
    .await
    .expect("stopped gateway should become unavailable before timeout")
}

pub(crate) fn assert_port_released(addr: SocketAddr) {
    let listener = std::net::TcpListener::bind(addr)
        .expect("stopped gateway should release its loopback port");
    drop(listener);
}

pub(crate) async fn assert_gateway_health(manager: &ServiceManager, addr: SocketAddr) {
    let manager_health = manager
        .health(LLM_SERVICE_NAME)
        .await
        .expect("manager should validate gateway health");
    assert_eq!(manager_health.service, LLM_SERVICE_NAME);
    assert_eq!(manager_health.status, HealthStatus::Healthy);
    assert_eq!(manager_health.version.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        manager_health.version.capability_version,
        LLM_CHAT_CAPABILITY_VERSION
    );

    let client_health = HttpLlmClient::new(format!("http://{addr}"))
        .health()
        .await
        .expect("typed client should validate the independent gateway process");
    assert_eq!(client_health, manager_health);
}
