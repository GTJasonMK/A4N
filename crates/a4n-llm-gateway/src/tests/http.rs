use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use a4n_capabilities::{CapabilityClient, HttpLlmClient, JsonRequest, LlmClient};
use a4n_llm::provider::ChatRequest;
use a4n_service_protocol::{
    HealthStatus, LLM_CHAT_CAPABILITY_VERSION, LLM_SERVICE_NAME, PROTOCOL_VERSION,
    ServiceErrorCode, ServiceRequest, ServiceStatus, ServiceTaskEventKind,
};
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::post,
};
use futures::{future::pending, stream};
use serde::Serialize;
use serde_json::{Value, json};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{Duration, timeout},
};

use super::{service_request, test_provider_config};
use crate::{app::build_router, contracts::LlmChatPayload};

mod errors;

struct TestHttpServer {
    addr: SocketAddr,
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
}

impl TestHttpServer {
    async fn start(router: Router) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("loopback test listener should bind");
        let addr = listener
            .local_addr()
            .expect("loopback test listener should have a local address");
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
            shutdown,
            task,
        }
    }

    fn client(&self) -> HttpLlmClient {
        HttpLlmClient::new(self.base_url())
    }

    fn base_url(&self) -> String {
        format!("http://{}", self.addr)
    }

    async fn stop(self) {
        let _ = self.shutdown.send(());
        timeout(Duration::from_secs(5), self.task)
            .await
            .expect("loopback HTTP server should stop before timeout")
            .expect("loopback HTTP server task should join")
            .expect("loopback HTTP server should stop cleanly");
    }
}

fn invalid_provider_payload() -> LlmChatPayload {
    LlmChatPayload {
        provider: super::HttpLlmProviderConfig {
            base_url: "not a provider URL".to_string(),
            api_key: Some("payload-provider-secret".to_string()),
            ..test_provider_config()
        },
        request: ChatRequest::system_user("system", "ping", None, None, None),
    }
}

fn json_request<T: Serialize>(request: ServiceRequest<T>) -> JsonRequest {
    serde_json::from_value(
        serde_json::to_value(request).expect("typed service request should serialize"),
    )
    .expect("typed service request should preserve the generic JSON envelope")
}

#[derive(Debug, Clone, PartialEq)]
struct FakeProviderRequest {
    authorization: Option<String>,
    body: Value,
}

type FakeProviderObservation = Arc<Mutex<Option<FakeProviderRequest>>>;

fn observe_fake_provider_request(
    observation: &FakeProviderObservation,
    headers: &HeaderMap,
    body: Value,
) {
    let authorization = headers
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .map(str::to_string);
    *observation
        .lock()
        .expect("fake provider observation lock should not be poisoned") =
        Some(FakeProviderRequest {
            authorization,
            body,
        });
}

fn fake_provider_router(observation: FakeProviderObservation) -> Router {
    Router::new()
        .route("/chat/completions", post(fake_chat_completions))
        .with_state(observation)
}

async fn fake_chat_completions(
    State(observation): State<FakeProviderObservation>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    observe_fake_provider_request(&observation, &headers, body);

    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hel\"},\"finish_reason\":null}]}\n\n",
            "data: {\"choices\":[{\"index\":1,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5}}\n\n",
            "data: [DONE]\n\n"
        ),
    )
}

#[derive(Clone)]
struct HangingFakeProviderState {
    observation: FakeProviderObservation,
    disconnect_signal: Arc<Mutex<Option<oneshot::Sender<()>>>>,
}

struct ProviderBodyDropSignal(Option<oneshot::Sender<()>>);

impl Drop for ProviderBodyDropSignal {
    fn drop(&mut self) {
        if let Some(signal) = self.0.take() {
            let _ = signal.send(());
        }
    }
}

fn hanging_fake_provider_router(
    observation: FakeProviderObservation,
    disconnect_signal: oneshot::Sender<()>,
) -> Router {
    Router::new()
        .route("/chat/completions", post(hanging_fake_chat_completions))
        .with_state(HangingFakeProviderState {
            observation,
            disconnect_signal: Arc::new(Mutex::new(Some(disconnect_signal))),
        })
}

async fn hanging_fake_chat_completions(
    State(state): State<HangingFakeProviderState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> impl IntoResponse {
    observe_fake_provider_request(&state.observation, &headers, body);
    let disconnect_signal = state
        .disconnect_signal
        .lock()
        .expect("provider disconnect signal lock should not be poisoned")
        .take()
        .expect("hanging fake provider should be called once");
    let body_stream = stream::unfold(
        (false, ProviderBodyDropSignal(Some(disconnect_signal))),
        |(first_sent, drop_signal)| async move {
            if first_sent {
                return pending::<
                    Option<(Result<Bytes, Infallible>, (bool, ProviderBodyDropSignal))>,
                >()
                .await;
            }
            Some((
                Ok(Bytes::from_static(
                    b"data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"first\"},\"finish_reason\":null}]}\n\n",
                )),
                (true, drop_signal),
            ))
        },
    );

    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        Body::from_stream(body_stream),
    )
}

#[tokio::test]
async fn typed_http_client_rejects_malformed_envelopes_before_provider_construction() {
    let server = TestHttpServer::start(build_router()).await;
    let client = server.client();

    let health = client
        .health()
        .await
        .expect("typed LLM client should validate the production health envelope");

    let mut root_without_project = service_request(invalid_provider_payload());
    root_without_project.project_id = None;
    root_without_project.root_project_id = Some(3);
    let root_response = client
        .chat(json_request(root_without_project))
        .await
        .expect("protocol rejection should be returned as a service response");

    let mut unredacted_snapshot = service_request(invalid_provider_payload());
    unredacted_snapshot.runtime_config_snapshot = Some(json!({
        "llm": {
            "api_key": "snapshot-secret",
            "base_url": "https://private-provider.example/v1"
        }
    }));
    let snapshot_response = client
        .chat(json_request(unredacted_snapshot))
        .await
        .expect("snapshot rejection should be returned as a service response");

    let mut incompatible_capability = service_request(invalid_provider_payload());
    incompatible_capability.capability_version = "llm-chat-v0".to_string();
    let capability_response = client.chat(json_request(incompatible_capability)).await;

    server.stop().await;

    assert_eq!(health.service, LLM_SERVICE_NAME);
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        health.version.capability_version,
        LLM_CHAT_CAPABILITY_VERSION
    );

    let capability_response =
        capability_response.expect("capability mismatch should be returned as a service response");
    for response in [&root_response, &snapshot_response, &capability_response] {
        let response_text =
            serde_json::to_string(response).expect("service response should serialize");
        assert!(!response_text.contains("payload-provider-secret"));
        assert!(!response_text.contains("not a provider URL"));
    }

    let snapshot_text =
        serde_json::to_string(&snapshot_response).expect("snapshot response should serialize");
    assert!(!snapshot_text.contains("snapshot-secret"));
    assert!(!snapshot_text.contains("private-provider.example"));

    assert_eq!(root_response.status, ServiceStatus::Error);
    assert_eq!(root_response.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(
        root_response.error.as_ref().map(|error| error.code.clone()),
        Some(ServiceErrorCode::ConfigurationError)
    );
    assert!(root_response.error.as_ref().is_some_and(|error| {
        error
            .message
            .contains("root_project_id requires project_id")
            && error.retryable == Some(false)
    }));

    assert_eq!(snapshot_response.status, ServiceStatus::Error);
    assert_eq!(snapshot_response.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(
        snapshot_response
            .error
            .as_ref()
            .map(|error| error.code.clone()),
        Some(ServiceErrorCode::ConfigurationError)
    );
    assert!(snapshot_response.error.as_ref().is_some_and(|error| {
        error.message.contains("unredacted sensitive fields") && error.retryable == Some(false)
    }));

    assert_eq!(capability_response.status, ServiceStatus::Error);
    assert_eq!(capability_response.trace_id.as_deref(), Some("trace-1"));
    assert_eq!(
        capability_response
            .error
            .as_ref()
            .map(|error| error.code.clone()),
        Some(ServiceErrorCode::VersionMismatch)
    );
    assert!(
        capability_response
            .error
            .as_ref()
            .is_some_and(|error| error.retryable == Some(false))
    );
}

#[tokio::test]
async fn typed_stream_client_roundtrips_fake_provider_chunks_and_final_usage() {
    let provider_observation = Arc::new(Mutex::new(None));
    let provider = TestHttpServer::start(fake_provider_router(provider_observation.clone())).await;
    let provider_base_url = provider.base_url();
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();

    let request = service_request(LlmChatPayload {
        provider: super::HttpLlmProviderConfig {
            base_url: provider_base_url.clone(),
            model: "fake-stream-model".to_string(),
            api_key: Some("fake-provider-secret".to_string()),
            timeout_secs: 5,
            request_options_json: None,
        },
        request: ChatRequest::system_user("system", "stream ping", None, Some(16), None)
            .with_stream_usage(),
    });
    let (event_tx, mut event_rx) = tokio::sync::mpsc::unbounded_channel();

    client
        .stream(json_request(request), event_tx)
        .await
        .expect("typed LLM stream should complete through the fake provider");
    let mut events = Vec::new();
    while let Some(event) = event_rx.recv().await {
        events.push(event);
    }

    gateway.stop().await;
    provider.stop().await;

    let provider_request = provider_observation
        .lock()
        .expect("fake provider observation lock should not be poisoned")
        .clone()
        .expect("gateway should call the fake provider once");
    assert_eq!(
        provider_request.authorization.as_deref(),
        Some("Bearer fake-provider-secret")
    );
    assert_eq!(provider_request.body["model"], json!("fake-stream-model"));
    assert_eq!(provider_request.body["stream"], json!(true));
    assert_eq!(
        provider_request.body["stream_options"]["include_usage"],
        json!(true)
    );

    assert_eq!(events.len(), 3);
    let events_text = serde_json::to_string(&events).expect("stream events should serialize");
    assert!(!events_text.contains("fake-provider-secret"));
    assert!(!events_text.contains(&provider_base_url));

    assert_eq!(events[0].task_id, "test-request");
    assert_eq!(events[0].event_kind, ServiceTaskEventKind::Chunk);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(events[0].chunk_index, Some(0));
    assert_eq!(events[0].chunk_delta.as_deref(), Some("hel"));
    assert_eq!(events[0].usage_delta, None);

    assert_eq!(events[1].event_kind, ServiceTaskEventKind::Chunk);
    assert_eq!(events[1].sequence, 2);
    assert_eq!(events[1].chunk_index, Some(1));
    assert_eq!(events[1].chunk_delta.as_deref(), Some("lo"));
    assert_eq!(events[1].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[1].is_truncated_finish_reason, Some(false));
    assert_eq!(
        events[1]
            .usage_delta
            .as_ref()
            .map(|usage| usage["total_tokens"].clone()),
        Some(json!(5))
    );

    assert_eq!(events[2].event_kind, ServiceTaskEventKind::Completed);
    assert_eq!(events[2].sequence, 3);
    assert_eq!(events[2].chunk_delta, None);
    assert_eq!(events[2].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[2].is_truncated_finish_reason, Some(false));
    assert_eq!(
        events[2]
            .final_usage
            .as_ref()
            .map(|usage| usage["total_tokens"].clone()),
        Some(json!(5))
    );
    assert_eq!(events[2].error, None);
}

#[tokio::test]
async fn closed_typed_event_receiver_drops_gateway_and_provider_http_streams() {
    let provider_observation = Arc::new(Mutex::new(None));
    let (provider_disconnected_tx, provider_disconnected_rx) = oneshot::channel();
    let provider = TestHttpServer::start(hanging_fake_provider_router(
        provider_observation.clone(),
        provider_disconnected_tx,
    ))
    .await;
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();
    let request = service_request(LlmChatPayload {
        provider: super::HttpLlmProviderConfig {
            base_url: provider.base_url(),
            model: "fake-cancel-model".to_string(),
            api_key: Some("fake-cancel-secret".to_string()),
            timeout_secs: 5,
            request_options_json: None,
        },
        request: ChatRequest::system_user("system", "cancel ping", None, Some(16), None),
    });
    let (event_tx, event_rx) = tokio::sync::mpsc::unbounded_channel();
    drop(event_rx);

    let error = timeout(
        Duration::from_secs(5),
        client.stream(json_request(request), event_tx),
    )
    .await
    .expect("typed stream should observe the closed receiver before timeout")
    .expect_err("closed event receiver must cancel the typed stream");
    timeout(Duration::from_secs(5), provider_disconnected_rx)
        .await
        .expect("provider response body should be dropped before timeout")
        .expect("provider disconnect signal should be delivered");

    gateway.stop().await;
    provider.stop().await;

    assert_eq!(error.code, ServiceErrorCode::Cancelled);
    assert_eq!(error.retryable, Some(false));
    assert!(error.message.contains("event receiver 已关闭"));
    let provider_request = provider_observation
        .lock()
        .expect("fake provider observation lock should not be poisoned")
        .clone()
        .expect("gateway should have started the fake provider stream");
    assert_eq!(provider_request.body["stream"], json!(true));
    assert_eq!(
        provider_request.authorization.as_deref(),
        Some("Bearer fake-cancel-secret")
    );
}
