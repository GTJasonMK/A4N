use std::{
    net::SocketAddr,
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};

use a4n_capabilities::{
    CapabilityClient, EVENT_QUEUE_CAPACITY, HttpLlmClient, JsonRequest, LlmClient,
};
use a4n_llm::provider::ChatRequest;
use a4n_service_manager::{
    ManagedServiceConfig, ManagedSidecarState, ServiceManager, ServiceManagerError,
};
use a4n_service_protocol::{
    CapabilityTaskEvent, HealthStatus, LLM_CHAT_CAPABILITY_VERSION, LLM_SERVICE_NAME,
    PROTOCOL_VERSION, SIDECAR_AUTH_TOKEN_ENV, SecretForwardingPolicy, ServiceRequestContext,
    TraceContext,
};
use a4n_test_support::process::SIDECAR_PROCESS_TEST_TOKEN;
use axum::{
    Json, Router,
    extract::State,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::{get, post},
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
    config.env.insert(
        SIDECAR_AUTH_TOKEN_ENV.to_string(),
        SIDECAR_PROCESS_TEST_TOKEN.to_string(),
    );
    config.startup_timeout_ms = 10_000;
    ServiceManager::new(vec![config]).expect("gateway sidecar config should be valid")
}

pub(crate) fn gateway_client(addr: SocketAddr) -> HttpLlmClient {
    HttpLlmClient::new_with_auth_token(format!("http://{addr}"), SIDECAR_PROCESS_TEST_TOKEN)
}

pub(crate) fn gateway_client_with_test_timeouts(addr: SocketAddr) -> HttpLlmClient {
    HttpLlmClient::new_with_auth_token(format!("http://{addr}"), SIDECAR_PROCESS_TEST_TOKEN)
        .with_test_timeouts(
            Duration::from_secs(1),
            Duration::from_secs(2),
            Duration::from_millis(100),
            Duration::from_secs(1),
        )
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
            .route("/models", get(fake_models))
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

async fn fake_models() -> Json<Value> {
    Json(json!({
        "data": [{"id": "process-fake-model", "owned_by": "process-test"}]
    }))
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
        if body["model"] == json!("process-burst-model") {
            let mut stream = String::new();
            for _ in 0..(EVENT_QUEUE_CAPACITY + 8) {
                stream.push_str(
                    "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"x\"},\"finish_reason\":null}]}\n\n",
                );
            }
            stream.push_str("data: [DONE]\n\n");
            return ([(header::CONTENT_TYPE, "text/event-stream")], stream).into_response();
        }
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
    gateway_request_with_policy(
        request_id,
        trace_id,
        provider_base_url,
        provider_model,
        Some("process-provider-secret"),
        SecretForwardingPolicy::local_user_secret(),
        request,
    )
}

pub(crate) fn gateway_remote_request_for_model(
    request_id: &str,
    trace_id: &str,
    provider_base_url: &str,
    provider_model: &str,
    api_key: Option<&str>,
    request: ChatRequest,
) -> JsonRequest {
    gateway_request_with_policy(
        request_id,
        trace_id,
        provider_base_url,
        provider_model,
        api_key,
        SecretForwardingPolicy::trusted_remote_no_secrets(),
        request,
    )
}

fn gateway_request_with_policy(
    request_id: &str,
    trace_id: &str,
    provider_base_url: &str,
    provider_model: &str,
    api_key: Option<&str>,
    secret_policy: SecretForwardingPolicy,
    request: ChatRequest,
) -> JsonRequest {
    let mut context = ServiceRequestContext::new(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: Some("process-integration".to_string()),
        },
        secret_policy,
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
                    "api_key": api_key,
                    "timeout_secs": 5,
                    "request_options_json": null
                },
                "request": request
            }),
        )
        .expect("process integration request context should be valid")
}

pub(crate) fn gateway_probe_request(
    request_id: &str,
    trace_id: &str,
    provider_base_url: &str,
) -> JsonRequest {
    ServiceRequestContext::local_user_secret(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: Some("process-integration".to_string()),
        },
    )
    .with_payload(
        LLM_CHAT_CAPABILITY_VERSION,
        json!({
            "provider": {
                "base_url": provider_base_url,
                "model": "process-fake-model",
                "api_key": "process-provider-secret",
                "timeout_secs": 5,
                "request_options_json": null
            },
            "request": ChatRequest::system_user("system", "probe ping", None, Some(16), None)
        }),
    )
    .expect("process probe request context should be valid")
}

pub(crate) fn gateway_models_request(
    request_id: &str,
    trace_id: &str,
    provider_base_url: &str,
) -> JsonRequest {
    ServiceRequestContext::local_user_secret(
        request_id,
        TraceContext {
            trace_id: trace_id.to_string(),
            parent_span_id: Some("process-integration".to_string()),
        },
    )
    .with_payload(
        LLM_CHAT_CAPABILITY_VERSION,
        json!({
            "provider": "openai",
            "base_url": provider_base_url,
            "api_key": "process-provider-secret"
        }),
    )
    .expect("process models request context should be valid")
}

pub(crate) async fn collect_stream_events(
    client: &impl LlmClient,
    request: JsonRequest,
) -> Vec<CapabilityTaskEvent> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(EVENT_QUEUE_CAPACITY);
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
            if let Err(error) = manager.health(LLM_SERVICE_NAME).await
                && manager
                    .process_status(LLM_SERVICE_NAME)
                    .await
                    .expect("gateway process status should remain readable")
                    .is_some_and(|status| status.state == ManagedSidecarState::Stopped)
            {
                // Health can fail before force-stop has finished reaping the child.
                return error;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("stopped gateway should become unavailable and finish stopping before timeout")
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

    let client_health = gateway_client(addr)
        .health()
        .await
        .expect("typed client should validate the independent gateway process");
    assert_eq!(client_health, manager_health);
}
