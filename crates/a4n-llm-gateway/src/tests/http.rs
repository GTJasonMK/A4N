use std::{
    convert::Infallible,
    net::SocketAddr,
    sync::{Arc, Mutex},
    time::Instant,
};

use a4n_capabilities::{CapabilityClient, HttpLlmClient, JsonRequest, LlmClient};
use a4n_llm::gateway::{LlmGatewayPayload, LlmGatewayProviderConfig};
use a4n_llm::provider::{ChatChunk, ChatRequest};
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind, HealthStatus, LLM_CHAT_CAPABILITY_VERSION,
    LLM_SERVICE_NAME, PROTOCOL_VERSION, SIDECAR_EVENT_QUEUE_CAPACITY, ServiceErrorCode,
    ServiceRequest,
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
use sha2::{Digest, Sha256};
use tokio::{
    sync::oneshot,
    task::JoinHandle,
    time::{Duration, timeout},
};

use super::{service_request, test_provider_config};
use crate::app::build_router;

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

async fn collect_stream_events(
    client: &impl LlmClient,
    request: JsonRequest,
) -> Vec<CapabilityTaskEvent> {
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
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

fn invalid_provider_payload() -> LlmGatewayPayload {
    LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
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
    let root_events = collect_stream_events(&client, json_request(root_without_project)).await;

    let mut unredacted_snapshot = service_request(invalid_provider_payload());
    unredacted_snapshot.runtime_config_snapshot = Some(json!({
        "llm": {
            "api_key": "snapshot-secret",
            "base_url": "https://private-provider.example/v1"
        }
    }));
    let snapshot_events = collect_stream_events(&client, json_request(unredacted_snapshot)).await;

    let mut incompatible_capability = service_request(invalid_provider_payload());
    incompatible_capability.capability_version = "llm-chat-v0".to_string();
    let capability_events =
        collect_stream_events(&client, json_request(incompatible_capability)).await;

    server.stop().await;

    assert_eq!(health.service, LLM_SERVICE_NAME);
    assert_eq!(health.status, HealthStatus::Healthy);
    assert_eq!(health.version.protocol_version, PROTOCOL_VERSION);
    assert_eq!(
        health.version.capability_version,
        LLM_CHAT_CAPABILITY_VERSION
    );

    for events in [&root_events, &snapshot_events, &capability_events] {
        let response_text = serde_json::to_string(events).expect("events should serialize");
        assert!(!response_text.contains("payload-provider-secret"));
        assert!(!response_text.contains("not a provider URL"));
    }

    let snapshot_text = serde_json::to_string(&snapshot_events).expect("events should serialize");
    assert!(!snapshot_text.contains("snapshot-secret"));
    assert!(!snapshot_text.contains("private-provider.example"));

    assert_eq!(root_events.len(), 1);
    assert_eq!(root_events[0].event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(
        root_events[0]
            .error
            .as_ref()
            .map(|error| error.code.clone()),
        Some(ServiceErrorCode::ConfigurationError)
    );
    assert!(root_events[0].error.as_ref().is_some_and(|error| {
        error
            .message
            .contains("root_project_id requires project_id")
            && error.retryable == Some(false)
    }));

    assert_eq!(snapshot_events.len(), 1);
    assert_eq!(
        snapshot_events[0].event_kind,
        CapabilityTaskEventKind::Failed
    );
    assert_eq!(
        snapshot_events[0]
            .error
            .as_ref()
            .map(|error| error.code.clone()),
        Some(ServiceErrorCode::ConfigurationError)
    );
    assert!(snapshot_events[0].error.as_ref().is_some_and(|error| {
        error.message.contains("unredacted sensitive fields") && error.retryable == Some(false)
    }));

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
        request: ChatRequest::system_user("system", "stream ping", None, Some(16), None)
            .with_stream_usage(),
    });
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);

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
        request: ChatRequest::system_user("system", "cancel ping", None, Some(16), None),
    });
    let (closed_tx, closed_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    drop(closed_rx);
    let early_error = timeout(
        Duration::from_secs(1),
        client.stream(json_request(request.clone()), closed_tx),
    )
    .await
    .expect("a closed receiver must cancel before opening the request")
    .expect_err("a closed receiver must not reach the provider");
    assert_eq!(early_error.code, ServiceErrorCode::Cancelled);
    assert!(provider_observation.lock().unwrap().is_none());

    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(SIDECAR_EVENT_QUEUE_CAPACITY);
    let stream = tokio::spawn(async move { client.stream(json_request(request), event_tx).await });
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

#[derive(Clone)]
struct BaselineFakeProviderState {
    chunks: Arc<Vec<String>>,
}

fn baseline_fake_provider_router(chunks: Vec<String>) -> Router {
    Router::new()
        .route("/chat/completions", post(baseline_fake_chat_completions))
        .with_state(BaselineFakeProviderState {
            chunks: Arc::new(chunks),
        })
}

async fn baseline_fake_chat_completions(
    State(state): State<BaselineFakeProviderState>,
) -> impl IntoResponse {
    let last_index = state.chunks.len().saturating_sub(1);
    let mut frames = state
        .chunks
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

async fn run_stream_baseline(
    chunks: Vec<String>,
) -> (
    Vec<a4n_service_protocol::CapabilityTaskEvent>,
    Duration,
    Duration,
) {
    let event_capacity = chunks.len() + 1;
    let provider = TestHttpServer::start(baseline_fake_provider_router(chunks)).await;
    let gateway = TestHttpServer::start(build_router()).await;
    let client = gateway.client();
    let request = service_request(LlmGatewayPayload {
        provider: LlmGatewayProviderConfig {
            provider: "openai".to_string(),
            base_url: provider.base_url(),
            model: "fake-baseline-model".to_string(),
            api_key: None,
            timeout_secs: 30,
            extra_body_json: None,
        },
        request: ChatRequest::system_user("system", "baseline", None, None, None),
    });
    let (event_tx, mut event_rx) = tokio::sync::mpsc::channel(event_capacity);
    let started_at = Instant::now();
    let mut producer =
        tokio::spawn(async move { client.stream(json_request(request), event_tx).await });

    let first_event = timeout(Duration::from_secs(10), event_rx.recv())
        .await
        .expect("baseline stream should publish its first event before timeout")
        .expect("baseline stream should not close before its first event");
    let first_event_latency = started_at.elapsed();

    timeout(Duration::from_secs(30), &mut producer)
        .await
        .expect("old unbounded event producer should finish while the consumer is paused")
        .expect("baseline stream task should join")
        .expect("baseline stream should complete");
    let producer_elapsed = started_at.elapsed();

    let mut events = vec![first_event];
    while let Some(event) = event_rx.recv().await {
        events.push(event);
    }

    gateway.stop().await;
    provider.stop().await;

    (events, first_event_latency, producer_elapsed)
}

fn sha256_hex(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}

#[tokio::test]
#[ignore = "manual stage-0 streaming baseline"]
async fn reports_stage_0_streaming_baseline() {
    let small_chunks = (0..10_000)
        .map(|index| {
            let byte = b'a' + (index % 26) as u8;
            String::from_utf8(vec![byte; index % 8 + 1]).expect("ASCII chunk should be UTF-8")
        })
        .collect::<Vec<_>>();
    let expected_small = small_chunks.concat();
    let (small_events, small_first_latency, small_producer_elapsed) =
        run_stream_baseline(small_chunks).await;
    let observed_small = small_events
        .iter()
        .filter(|event| event.event_kind == CapabilityTaskEventKind::Chunk)
        .map(|event| event_chunk(event).content.expect("content chunk"))
        .collect::<String>();
    let small_chunk_events = small_events
        .iter()
        .filter(|event| event.event_kind == CapabilityTaskEventKind::Chunk)
        .count();

    assert_eq!(small_chunk_events, 10_000);
    assert_eq!(small_events.len(), 10_001);
    assert_eq!(observed_small, expected_small);
    assert_eq!(sha256_hex(&observed_small), sha256_hex(&expected_small));
    assert_eq!(
        small_events.last().map(|event| &event.event_kind),
        Some(&CapabilityTaskEventKind::Completed)
    );
    eprintln!(
        "BASELINE_STREAM_10000 first_event_us={} producer_elapsed_ms={} upstream_chunks=10000 chunk_events={} total_events={} final_bytes={} sha256={} slow_consumer=producer_completed_while_receiver_paused",
        small_first_latency.as_micros(),
        small_producer_elapsed.as_millis(),
        small_chunk_events,
        small_events.len(),
        observed_small.len(),
        sha256_hex(&observed_small),
    );

    let one_mib_chunks = vec!["x".repeat(1024); 1024];
    let expected_one_mib = one_mib_chunks.concat();
    let (one_mib_events, one_mib_first_latency, one_mib_producer_elapsed) =
        run_stream_baseline(one_mib_chunks).await;
    let observed_one_mib = one_mib_events
        .iter()
        .filter(|event| event.event_kind == CapabilityTaskEventKind::Chunk)
        .map(|event| event_chunk(event).content.expect("content chunk"))
        .collect::<String>();
    let one_mib_chunk_events = one_mib_events
        .iter()
        .filter(|event| event.event_kind == CapabilityTaskEventKind::Chunk)
        .count();

    assert_eq!(one_mib_chunk_events, 1024);
    assert_eq!(one_mib_events.len(), 1025);
    assert_eq!(observed_one_mib, expected_one_mib);
    assert_eq!(sha256_hex(&observed_one_mib), sha256_hex(&expected_one_mib));
    eprintln!(
        "BASELINE_STREAM_1M first_event_us={} producer_elapsed_ms={} upstream_chunks=1024 chunk_events={} total_events={} final_bytes={} sha256={} slow_consumer=producer_completed_while_receiver_paused",
        one_mib_first_latency.as_micros(),
        one_mib_producer_elapsed.as_millis(),
        one_mib_chunk_events,
        one_mib_events.len(),
        observed_one_mib.len(),
        sha256_hex(&observed_one_mib),
    );
}
