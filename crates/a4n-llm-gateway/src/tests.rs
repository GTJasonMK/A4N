use a4n_core::error::AppError;
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
use crate::protocol::validate_provider_secret_policy;
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

    let output = probe_existing_provider(&provider, Some(request))
        .await
        .expect("probe with request should collect chat_stream");

    assert!(output.ok);
    assert_eq!(output.response_preview.as_deref(), Some("probe ok"));
    assert_eq!(output.finish_reason.as_deref(), Some("stop"));
    assert_eq!(
        output.usage.as_ref().map(|usage| usage.total_tokens),
        Some(5)
    );
    assert_eq!(provider.stream_call_count(), 1);
    assert_eq!(provider.test_connection_call_count(), 0);
}

#[tokio::test]
async fn probe_rejects_empty_stream_response() {
    let provider = CountingLlmProvider::new(ChatResponse {
        message: ChatMessage::assistant("   "),
        usage: None,
        finish_reason: Some("stop".to_string()),
    });
    let request = ChatRequest::system_user("system", "ping", Some(0.0), Some(16), None);

    let error = probe_existing_provider(&provider, Some(request))
        .await
        .expect_err("empty probe response must fail");

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

#[test]
fn secret_policy_allows_deny_all_when_no_api_key_is_forwarded() {
    validate_provider_secret_policy(None, &SecretForwardingPolicy::deny_all())
        .expect("no API key should not require user secret forwarding");
    validate_provider_secret_policy(Some("  "), &SecretForwardingPolicy::deny_all())
        .expect("blank API key should not count as forwarded secret");
}

#[test]
fn secret_policy_rejects_api_key_for_trusted_remote_sidecar() {
    let policy = SecretForwardingPolicy {
        scope: a4n_service_protocol::SecretMaterialScope::TrustedRemote,
        allow_system_secret: false,
        allow_user_secret: true,
        allow_upload_token: false,
    };

    let error = validate_provider_secret_policy(Some("secret-key"), &policy)
        .expect_err("remote sidecar must not receive forwarded user secret");

    assert_eq!(error.code, ServiceErrorCode::Unauthorized);
    assert!(error.message.contains("local_sidecar_only"));
}

#[test]
fn secret_policy_rejects_api_key_with_unrelated_secret_permissions() {
    for policy in [
        SecretForwardingPolicy {
            allow_upload_token: true,
            ..SecretForwardingPolicy::local_user_secret()
        },
        SecretForwardingPolicy {
            allow_system_secret: true,
            ..SecretForwardingPolicy::local_user_secret()
        },
    ] {
        let error = validate_provider_secret_policy(Some("secret-key"), &policy)
            .expect_err("provider API key must use the exact user-secret-only policy");

        assert_eq!(error.code, ServiceErrorCode::Unauthorized);
        assert!(error.message.contains("user-secret-only"));
    }
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
async fn llm_stream_emits_chunks_and_completed_final_usage() {
    let usage = UsageInfo {
        prompt_tokens: 11,
        completion_tokens: 7,
        total_tokens: 18,
        prompt_cache_hit_tokens: Some(3),
        prompt_cache_miss_tokens: Some(8),
    };
    let usage_json = serde_json::to_value(&usage).expect("usage should serialize");
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
    assert_eq!(events[0].chunk_index, Some(0));
    assert_eq!(events[0].chunk_delta.as_deref(), Some("hel"));
    assert_eq!(events[0].usage_delta, None);

    assert_eq!(events[1].event_kind, CapabilityTaskEventKind::Chunk);
    assert_eq!(events[1].sequence, 2);
    assert_eq!(events[1].chunk_index, Some(1));
    assert_eq!(events[1].chunk_delta.as_deref(), Some("lo"));
    assert_eq!(events[1].usage_delta, Some(usage_json.clone()));
    assert_eq!(events[1].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[1].is_truncated_finish_reason, Some(false));

    assert_eq!(events[2].event_kind, CapabilityTaskEventKind::Completed);
    assert_eq!(events[2].sequence, 3);
    assert_eq!(events[2].chunk_delta, None);
    assert_eq!(events[2].usage_delta, None);
    assert_eq!(events[2].final_usage, Some(usage_json));
    assert_eq!(events[2].finish_reason.as_deref(), Some("stop"));
    assert_eq!(events[2].is_truncated_finish_reason, Some(false));
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
    assert_eq!(events[0].final_usage, None);
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
