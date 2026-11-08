use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
};

use a4n_llm_client::{HttpLlmClient, LlmClient};
use a4n_llm_provider_contract::{ChatChunk, ChatRequest};
use a4n_llm_sidecar_contract::{
    LlmChatRequest, LlmGatewayPayload as WireGatewayPayload, LlmGatewayProviderConfig,
};
use a4n_service_client::ServiceClient;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, HealthStatus, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, PROTOCOL_VERSION, SIDECAR_EVENT_QUEUE_CAPACITY, ServiceErrorCode,
};
use a4n_test_support::process::collect_llm_stream_events as collect_stream_events;
use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::State,
    http::{HeaderMap, header},
    response::IntoResponse,
    routing::post,
};
use futures::{future::pending, stream};
use serde_json::{Value, json};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{Duration, timeout},
};

use super::{service_request, test_provider_config};
use crate::app::build_router;

mod errors;

type LlmGatewayPayload = WireGatewayPayload<LlmChatRequest>;

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

fn invalid_provider_payload() -> LlmGatewayPayload {
    LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: "not a provider URL".to_string(),
            api_key: Some("payload-provider-secret".to_string()),
            ..test_provider_config()
        },
        request: ChatRequest::system_user("system", "ping", None, None).into(),
    }
}

fn event_chunk(event: &CapabilityTaskEvent) -> ChatChunk {
    serde_json::from_value(event.payload.clone().expect("chunk payload"))
        .expect("typed chunk payload")
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
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"lo\"},\"finish_reason\":\"stop\"}],\"usage\":{\"prompt_tokens\":2,\"completion_tokens\":3,\"total_tokens\":5}}\n\n",
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
    let mut unredacted_snapshot = service_request(invalid_provider_payload());
    unredacted_snapshot.runtime_config_snapshot = Some(json!({
        "llm": {
            "api_key": "snapshot-secret",
            "base_url": "https://private-provider.example/v1"
        }
    }));
    for (request, message) in [
        (root_without_project, "root_project_id requires project_id"),
        (unredacted_snapshot, "unredacted sensitive fields"),
    ] {
        let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
        let error = client
            .stream(request, sender)
            .await
            .expect_err("invalid context must fail before opening an event stream");
        assert_eq!(error.code, ServiceErrorCode::ConfigurationError);
        assert_eq!(error.retryable, Some(false));
        assert!(error.message.contains(message));
        assert!(
            receiver.recv().await.is_none(),
            "client-side rejection must not invent a server event"
        );
        let serialized = serde_json::to_string(&error).expect("error should serialize");
        for private in [
            "payload-provider-secret",
            "not a provider URL",
            "snapshot-secret",
            "private-provider.example",
        ] {
            assert!(!serialized.contains(private));
        }
    }

    let mut incompatible_capability = service_request(invalid_provider_payload());
    incompatible_capability.capability_version = "llm-chat-v0".to_string();
    let capability_events = collect_stream_events(&client, incompatible_capability).await;

    server.stop().await;

    assert_eq!(health.service, LLM_SERVICE_NAME);
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        health.version.capability_version,
        LLM_CHAT_CAPABILITY_VERSION
    );

    let response_text = serde_json::to_string(&capability_events).expect("events should serialize");
    assert!(!response_text.contains("payload-provider-secret"));
    assert!(!response_text.contains("not a provider URL"));

    assert_eq!(capability_events.len(), 1);
    assert_eq!(
        capability_events[0].event_kind,
        CapabilityTaskEventKind::Failed
    );
    assert_eq!(
        capability_events[0]
            .error
            .as_ref()
            .map(|error| error.code.clone()),
        Some(ServiceErrorCode::VersionMismatch)
    );
    assert!(
        capability_events[0]
            .error
            .as_ref()
            .is_some_and(|error| error.retryable == Some(false))
    );
}

#[tokio::test]
async fn legacy_tool_id_protocol_is_rejected_before_provider_request() {
    let observation = Arc::new(Mutex::new(None));
    let provider = TestHttpServer::start(fake_provider_router(observation.clone())).await;
    let gateway = TestHttpServer::start(build_router()).await;
    let mut request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            base_url: provider.base_url(),
            ..test_provider_config()
        },
        request: ChatRequest::system_user("system", "ping", None, Some(16)).into(),
    });
    request.capability_version = "llm-chat-v9".into();
    let events = collect_stream_events(&gateway.client(), request).await;
    gateway.stop().await;
    provider.stop().await;

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Failed);
    let error = events[0].error.as_ref().unwrap();
    assert_eq!(error.code, ServiceErrorCode::VersionMismatch);
    assert_eq!(error.retryable, Some(false));
    assert!(
        observation.lock().unwrap().is_none(),
        "version rejection must not call a model provider"
    );
}

#[tokio::test]
async fn typed_stream_client_roundtrips_fake_provider_chunks() {
    let provider_observation = Arc::new(Mutex::new(None));
    let provider = TestHttpServer::start(fake_provider_router(provider_observation.clone())).await;
    let provider_base_url = provider.base_url();
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();

    let request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: provider_base_url.clone(),
            model: "fake-stream-model".to_string(),
            api_key: Some("fake-provider-secret".to_string()),
            timeout_secs: 5,
            extra_body_json: None,
        },
        request: ChatRequest::system_user("system", "stream ping", None, Some(16))
            .with_stream_usage()
            .into(),
    });
    let events = collect_stream_events(&client, request).await;

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
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[0].sequence, 1);
    let first = event_chunk(&events[0]);
    assert_eq!(first.index, 0);
    assert_eq!(first.content.as_deref(), Some("hel"));
    assert!(first.usage.is_none());

    assert_eq!(events[1].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[1].sequence, 2);
    let second = event_chunk(&events[1]);
    assert_eq!(second.index, 0);
    assert_eq!(second.content.as_deref(), Some("lo"));
    assert_eq!(second.finish_reason.as_deref(), Some("stop"));
    assert_eq!(
        second.usage.as_ref().map(|usage| usage.total_tokens),
        Some(5)
    );

    assert_eq!(events[2].event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(events[2].sequence, 3);
    assert_eq!(events[2].payload, None);
    assert_eq!(events[2].error, None);
}

#[tokio::test]
async fn closed_receiver_cancels_before_and_during_provider_streams() {
    let provider_observation = Arc::new(Mutex::new(None));
    let (provider_disconnected_tx, provider_disconnected_rx) = oneshot::channel();
    let provider = TestHttpServer::start(hanging_fake_provider_router(
        provider_observation.clone(),
        provider_disconnected_tx,
    ))
    .await;
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();
    let request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            model: "fake-cancel-model".to_string(),
            api_key: Some("fake-cancel-secret".to_string()),
            timeout_secs: 5,
            extra_body_json: None,
        },
        request: ChatRequest::system_user("system", "cancel ping", None, Some(16)).into(),
    });
    let (closed_tx, closed_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    drop(closed_rx);
    let early_error = timeout(
        Duration::from_secs(1),
        client.stream(request.clone(), closed_tx),
    )
    .await
    .expect("a closed receiver must cancel before opening the request")
    .expect_err("a closed receiver must not reach the provider");
    assert_eq!(early_error.code, ServiceErrorCode::Cancelled);
    assert!(provider_observation.lock().unwrap().is_none());

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let stream = tokio::spawn(async move { client.stream(request, event_tx).await });
    let first = timeout(Duration::from_secs(5), event_rx.recv())
        .await
        .expect("provider should start streaming before cancellation")
        .expect("first provider chunk should reach the receiver");
    assert_eq!(first.event_kind, CapabilityTaskEventKind::Chunk);
    drop(event_rx);

    let error = timeout(Duration::from_secs(5), stream)
        .await
        .expect("typed stream should observe the closed receiver before timeout")
        .expect("stream task should join")
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

fn chunked_provider_router(chunks: Vec<String>) -> Router {
    Router::new()
        .route("/chat/completions", post(chunked_chat_completions))
        .with_state(Arc::new(chunks))
}

async fn chunked_chat_completions(State(chunks): State<Arc<Vec<String>>>) -> impl IntoResponse {
    let last_index = chunks.len().saturating_sub(1);
    let mut frames = chunks
        .iter()
        .enumerate()
        .map(|(index, content)| {
            let finish_reason = (index == last_index).then_some("stop");
            let payload = json!({
                "choices": [{
                    "index": 0,
                    "delta": {"content": content},
                    "finish_reason": finish_reason,
                }]
            });
            Ok::<_, Infallible>(Bytes::from(format!("data: {payload}\n\n")))
        })
        .collect::<Vec<_>>();
    frames.push(Ok(Bytes::from_static(b"data: [DONE]\n\n")));

    (
        [(header::CONTENT_TYPE, "text/event-stream")],
        Body::from_stream(stream::iter(frames)),
    )
}

#[tokio::test]
async fn typed_stream_preserves_large_output_with_a_bounded_event_queue() {
    let small_chunks = (0..10_000)
        .map(|index| {
            let byte = b'a' + (index % 26) as u8;
            String::from_utf8(vec![byte; index % 8 + 1]).expect("ASCII chunk should be UTF-8")
        })
        .collect::<Vec<_>>();
    for chunks in [small_chunks, vec!["x".repeat(1024); 1024]] {
        let expected = chunks.concat();
        let chunk_count = chunks.len();
        assert!(chunk_count > SIDECAR_EVENT_QUEUE_CAPACITY);
        let provider = TestHttpServer::start(chunked_provider_router(chunks)).await;
        let gateway = TestHttpServer::start(build_router()).await;
        let events = timeout(
            Duration::from_secs(30),
            collect_stream_events(
                &gateway.client(),
                service_request(LlmGatewayPayload {
                    provider: LlmGatewayProviderConfig {
                        base_url: provider.base_url(),
                        timeout_secs: 30,
                        ..test_provider_config()
                    },
                    request: ChatRequest::system_user("system", "large stream", None, None).into(),
                }),
            ),
        )
        .await
        .expect("bounded event collection must drain without deadlocking");
        let observed = events
            .iter()
            .filter(|event| event.event_kind == CapabilityTaskEventKind::Chunk)
            .map(|event| event_chunk(event).content.expect("content chunk"))
            .collect::<String>();
        assert_eq!(observed, expected);
        assert_eq!(events.len(), chunk_count + 1);
        assert_eq!(
            events.last().map(|event| &event.event_kind),
            Some(&CapabilityTaskEventKind::Completed)
        );
        assert!(
            events
                .iter()
                .enumerate()
                .all(|(index, event)| event.sequence == index as u64 + 1)
        );
        gateway.stop().await;
        provider.stop().await;
    }
}
