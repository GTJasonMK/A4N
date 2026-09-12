use a4n_error::AppError;
use a4n_llm::gateway::{LlmGatewayPayload, LlmGatewayProviderConfig};
use a4n_llm::provider::{ChatChunk, ChatMessage, ChatRequest, ChatResponse, UsageInfo};
use a4n_model_discovery::ModelDiscoveryRequest;
use a4n_service_protocol::{
    CapabilityTaskEvent, CapabilityTaskEventKind,
    LLM_CHAT_CAPABILITY_VERSION as LLM_CAPABILITY_VERSION, PROTOCOL_VERSION, ServiceErrorCode,
    ServiceRequest, ServiceStatus,
};
use axum::{Json, body::Bytes};
use futures::{Stream, StreamExt, stream};

use crate::handlers::{discover_models, llm_event_stream, probe_existing_provider, stream_chat};
use a4n_service_protocol::{SchemaVersion, SecretForwardingPolicy, TraceContext};
use a4n_test_support::llm::CountingLlmProvider;
use std::convert::Infallible;
use std::time::Duration;

mod http;

fn test_provider_config() -> LlmGatewayProviderConfig {
    LlmGatewayProviderConfig {
        provider: "openai".to_string(),
        base_url: "http://127.0.0.1:1".to_string(),
        model: "test-model".to_string(),
        api_key: None,
        timeout_secs: 1,
        extra_body_json: None,
    }
}

fn test_provider_config_with_api_key() -> LlmGatewayProviderConfig {
    LlmGatewayProviderConfig {
        provider: "openai".to_string(),
        api_key: Some("secret-key".to_string()),
        ..test_provider_config()
    }
}

fn service_request<T>(payload: T) -> ServiceRequest<T> {
    ServiceRequest {
        request_id: "test-request".to_string(),
        trace_context: TraceContext {
            trace_id: "trace-1".to_string(),
            parent_span_id: None,
        },
        schema_version: SchemaVersion::default(),
        user_id: None,
        project_id: Some(7),
        root_project_id: Some(3),
        runtime_config_snapshot: None,
        secret_policy: SecretForwardingPolicy::local_user_secret(),
        capability_version: LLM_CAPABILITY_VERSION.to_string(),
        payload,
    }
}

#[tokio::test]
async fn probe_with_request_calls_stream_once() {
    let provider = CountingLlmProvider::default();
    let request = ChatRequest::system_user("system", "ping", Some(0.0), Some(16), None);

    let output = probe_existing_provider(&provider, request)
        .await
        .expect("probe with request should collect chat_stream");

    assert_eq!(output.response_preview, "probe ok");
    assert_eq!(output.finish_reason.as_deref(), Some("stop"));
    assert_eq!(
        output.usage.as_ref().map(|usage| usage.total_tokens),
        Some(5)
    );
    assert_eq!(provider.stream_call_count(), 1);
}

#[tokio::test]
async fn probe_rejects_empty_stream_response() {
    let provider = CountingLlmProvider::new(ChatResponse {
        message: ChatMessage::assistant("   "),
        usage: None,
        finish_reason: Some("stop".to_string()),
        reasoning_content: None,
    });
    let request = ChatRequest::system_user("system", "ping", Some(0.0), Some(16), None);

    let error = probe_existing_provider(&provider, request)
        .await
        .err()
        .expect("empty probe response must fail");

    assert!(error.to_string().contains("未收到有效响应"));
}

#[tokio::test]
async fn models_reject_api_key_when_user_secret_forwarding_is_denied() {
    let mut request = service_request(ModelDiscoveryRequest {
        provider: "openai".to_string(),
        base_url: "http://127.0.0.1:1/v1".to_string(),
        api_key: Some("secret-key".to_string()),
    });
    request.secret_policy = SecretForwardingPolicy::deny_all();

    let Json(response) = discover_models(Json(request)).await;

    assert_eq!(response.status, ServiceStatus::Error);
    assert_eq!(
        response.error.as_ref().map(|error| error.code.clone()),
        Some(ServiceErrorCode::Unauthorized)
    );
    assert!(response.payload.is_none());
}

#[tokio::test]
async fn stream_rejects_api_key_when_user_secret_forwarding_is_denied() {
    let mut request = service_request(LlmGatewayPayload {
        provider: test_provider_config_with_api_key(),
        request: ChatRequest::system_user("system", "ping", None, None, None),
    });
    request.secret_policy = SecretForwardingPolicy::deny_all();

    let response = stream_chat(Json(request)).await;
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("failed event response body should be readable");
    let event: CapabilityTaskEvent =
        serde_json::from_slice(&body).expect("failed event should deserialize");

    assert_eq!(event.event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(
        event.error.as_ref().map(|error| error.code.clone()),
        Some(ServiceErrorCode::Unauthorized)
    );
}

#[tokio::test]
async fn stream_response_headers_do_not_wait_for_provider_connection() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("fake provider listener should bind");
    let addr = listener
        .local_addr()
        .expect("fake provider listener should expose its address");
    let provider = tokio::spawn(async move {
        let _connection = listener
            .accept()
            .await
            .expect("provider request connection");
        std::future::pending::<()>().await;
    });
    let mut config = test_provider_config();
    config.base_url = format!("http://{addr}");
    let request = service_request(LlmGatewayPayload {
        provider: config,
        request: ChatRequest::system_user("system", "ping", None, None, None),
    });

    let response = tokio::time::timeout(Duration::from_millis(50), stream_chat(Json(request)))
        .await
        .expect("gateway must return stream headers before connecting to the provider");

    drop(response);
    provider.abort();
}

#[tokio::test]
async fn stream_rejects_incompatible_protocol_version_as_failed_event() {
    let mut request = service_request(LlmGatewayPayload {
        provider: test_provider_config(),
        request: ChatRequest::system_user("system", "ping", None, None, None),
    });
    request.schema_version.protocol = PROTOCOL_VERSION + 1;

    let response = stream_chat(Json(request)).await;
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .expect("failed event response body should be readable");
    let event: CapabilityTaskEvent =
        serde_json::from_slice(&body).expect("failed event should deserialize");

    assert_eq!(event.event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(event.sequence, 1);
    assert_eq!(
        event.error.as_ref().map(|error| error.code.clone()),
        Some(ServiceErrorCode::VersionMismatch)
    );
}

#[tokio::test]
async fn llm_stream_emits_typed_chunks_and_completed_marker() {
    let usage = UsageInfo {
        prompt_tokens: 11,
        completion_tokens: 7,
        total_tokens: 18,
        prompt_cache_hit_tokens: Some(3),
        prompt_cache_miss_tokens: Some(8),
    };
    let chunks = stream::iter(vec![
        Ok(ChatChunk {
            content: Some("hel".to_string()),
            reasoning_content: None,
            tool_calls: None,
            finish_reason: None,
            index: 0,
            usage: None,
        }),
        Ok(ChatChunk {
            content: Some("lo".to_string()),
            reasoning_content: None,
            tool_calls: None,
            finish_reason: Some("stop".to_string()),
            index: 1,
            usage: Some(usage),
        }),
    ]);

    let events = collect_events(llm_event_stream("llm-task".to_string(), Box::pin(chunks))).await;

    assert_eq!(events.len(), 3);
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[0].sequence, 1);
    let first = event_chunk(&events[0]);
    assert_eq!(first.index, 0);
    assert_eq!(first.content.as_deref(), Some("hel"));
    assert!(first.usage.is_none());

    assert_eq!(events[1].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[1].sequence, 2);
    let second = event_chunk(&events[1]);
    assert_eq!(second.index, 1);
    assert_eq!(second.content.as_deref(), Some("lo"));
    assert_eq!(
        second.usage.as_ref().map(|usage| usage.total_tokens),
        Some(18)
    );
    assert_eq!(second.finish_reason.as_deref(), Some("stop"));

    assert_eq!(events[2].event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(events[2].sequence, 3);
    assert_eq!(events[2].payload, None);
    assert_eq!(events[2].error, None);
}

#[tokio::test]
async fn llm_stream_error_emits_failed_event_without_completed() {
    let chunks = stream::iter(vec![Err(AppError::LlmError("provider failed".to_string()))]);

    let events = collect_events(llm_event_stream("llm-task".to_string(), Box::pin(chunks))).await;

    assert_eq!(events.len(), 1);
    assert_eq!(events[0].event_kind, CapabilityTaskEventKind::Failed);
    assert_eq!(events[0].sequence, 1);
    assert_eq!(
        events[0].error.as_ref().map(|error| error.code.clone()),
        Some(ServiceErrorCode::ProviderError)
    );
    assert_eq!(events[0].payload, None);
}

async fn collect_events(
    events: impl Stream<Item = Result<Bytes, Infallible>> + Send,
) -> Vec<CapabilityTaskEvent> {
    events
        .map(|line| {
            let line = line.expect("event stream uses infallible line items");
            serde_json::from_slice::<CapabilityTaskEvent>(&line)
                .expect("event line should deserialize")
        })
        .collect::<Vec<_>>()
        .await
}

fn event_chunk(event: &CapabilityTaskEvent) -> ChatChunk {
    serde_json::from_value(event.payload.clone().expect("chunk payload"))
        .expect("typed chunk payload")
}
