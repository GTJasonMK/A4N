use std::sync::Arc;

use a4n_domain::DEFAULT_LLM_MAX_OUTPUT_TOKENS;
use a4n_error::AppError;
use a4n_llm::provider::{
    ChatChunk, ChatMessage, ChatRequest, LlmTraceContext, MessageRole, ToolCall, UsageInfo,
};
use a4n_provider_contract::LlmProviderConfig;
use a4n_test_support::capability::RecordingLlmClient;

use crate::dynamic_llm::trace::validated_authoritative_trace_root_project_id;

use super::budget::{requested_max_tokens, resolve_effective_llm_preview_budget_for_config};
use super::config::{probe_llm_via_sidecar, require_sidecar_business_trace};
use super::contracts::{DynamicLlmProvider, ResolvedLlmModel};

mod support;
use support::TestProviderPort;

#[test]
fn max_tokens_cap_accepts_missing_or_within_limit() {
    assert!(
        requested_max_tokens(None, DEFAULT_LLM_MAX_OUTPUT_TOKENS, "deepseek-chat")
            .is_ok_and(|value| value == DEFAULT_LLM_MAX_OUTPUT_TOKENS)
    );
    assert!(
        requested_max_tokens(
            Some(DEFAULT_LLM_MAX_OUTPUT_TOKENS),
            DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            "deepseek-chat",
        )
        .is_ok_and(|value| value == DEFAULT_LLM_MAX_OUTPUT_TOKENS)
    );
}

#[test]
fn max_tokens_cap_rejects_explicit_request_above_model_limit() {
    let err = requested_max_tokens(Some(12_288), DEFAULT_LLM_MAX_OUTPUT_TOKENS, "deepseek-chat")
        .expect_err("explicit output limit must not exceed model capability");

    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("max_tokens=12288"));
    assert!(err.to_string().contains("model_max_output_tokens=8192"));
}

#[test]
fn configured_large_output_cap_survives_runtime_budget_resolution() {
    let mut model = test_llm_model();
    model.max_context_tokens = 1_048_576;
    model.max_output_tokens = 393_216;
    let request = ChatRequest::new(vec![ChatMessage::user("ping")], None, None);

    let budget = resolve_effective_llm_preview_budget_for_config(model.clone(), &request)
        .expect("large configured output cap should be accepted");

    assert_eq!(budget.max_output_tokens, 393_216);
    assert_eq!(budget.output_token_reserve, 393_216);
    assert_eq!(
        requested_max_tokens(request.max_tokens, 393_216, "test-model").expect("request budget"),
        393_216
    );
    assert_eq!(model.max_output_tokens, 393_216);
}

#[test]
fn resolved_model_rejects_remote_http_with_api_key() {
    let mut model = test_llm_model();
    model.base_url = "http://api.example/v1".to_string();

    let error = ResolvedLlmModel::new(model)
        .expect_err("runtime snapshots must not send credentials over remote HTTP");

    assert!(error.to_string().contains("必须使用 HTTPS"));
}

#[tokio::test]
async fn llm_connectivity_uses_probe_without_chat_fallback() {
    let client = Arc::new(RecordingLlmClient::new(successful_probe_response()));

    let output = probe_llm_via_sidecar(
        client.clone(),
        ResolvedLlmModel::new(test_llm_model()).expect("resolved model"),
        test_chat_request(),
    )
    .await
    .expect("LLM probe");

    assert_eq!(output.response_preview, "pong");
    assert_eq!(client.probe_call_count(), 1);
    let request = client.last_probe_request().expect("recorded probe request");
    assert!(request.secret_policy.is_local_user_secret_only());
    assert_eq!(request.payload["provider"]["model"], "test-model");
    assert_eq!(request.payload["provider"]["api_key"], "test-key");
    assert_eq!(request.payload["request"]["messages"][0]["content"], "ping");
}

#[tokio::test]
async fn sidecar_chat_keeps_incremental_tool_calls_and_tail_usage() {
    use a4n_llm_client::{JsonRequest, LlmClient, LlmProbeResponse};
    use a4n_model_discovery_contract::{ModelDiscoveryRequest, ModelDiscoveryResult};
    use a4n_service_client::{ServiceClient, ServiceClientResult};
    use a4n_service_protocol::{
        CapabilityTaskEvent, ServiceHealth, ServiceRequest, ServiceResponse,
    };
    use async_trait::async_trait;

    struct ToolStreamClient;

    #[async_trait]
    impl ServiceClient for ToolStreamClient {
        async fn health(&self) -> ServiceClientResult<ServiceHealth> {
            unreachable!("health is not used")
        }
    }

    #[async_trait]
    impl LlmClient for ToolStreamClient {
        async fn stream(
            &self,
            request: JsonRequest,
            events: tokio::sync::mpsc::Sender<CapabilityTaskEvent>,
        ) -> ServiceClientResult<()> {
            let chunks = [
                ChatChunk {
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "call-1".into(),
                        name: "locate".into(),
                        arguments: serde_json::Value::String("{\"project_id\":".into()),
                        stream_index: Some(0),
                        provider_metadata: None,
                    }]),
                    finish_reason: None,
                    index: 0,
                    usage: None,
                },
                ChatChunk {
                    content: None,
                    reasoning_content: None,
                    tool_calls: Some(vec![ToolCall {
                        id: "__stream_tool_0".into(),
                        name: String::new(),
                        arguments: serde_json::Value::String("7}".into()),
                        stream_index: Some(0),
                        provider_metadata: None,
                    }]),
                    finish_reason: Some("tool_calls".into()),
                    index: 0,
                    usage: None,
                },
                ChatChunk {
                    content: None,
                    reasoning_content: None,
                    tool_calls: None,
                    finish_reason: None,
                    index: 0,
                    usage: Some(UsageInfo {
                        prompt_tokens: 2,
                        completion_tokens: 3,
                        total_tokens: 5,
                        prompt_cache_hit_tokens: None,
                        prompt_cache_miss_tokens: None,
                        prompt_cache_write_tokens: None,
                    }),
                },
            ];
            for (index, chunk) in chunks.into_iter().enumerate() {
                events
                    .send(CapabilityTaskEvent::chunk(
                        request.request_id.clone(),
                        index as u64 + 1,
                        chunk,
                        "test chunk",
                    )?)
                    .await
                    .expect("event receiver should stay open");
            }
            events
                .send(CapabilityTaskEvent::completed_empty(request.request_id, 4))
                .await
                .expect("event receiver should stay open");
            Ok(())
        }

        async fn probe(&self, _request: JsonRequest) -> ServiceClientResult<LlmProbeResponse> {
            unreachable!("probe is not used")
        }

        async fn discover_models(
            &self,
            _request: ServiceRequest<ModelDiscoveryRequest>,
        ) -> ServiceClientResult<ServiceResponse<ModelDiscoveryResult>> {
            unreachable!("model discovery is not used")
        }
    }

    let port = Arc::new(TestProviderPort::default());
    let provider = DynamicLlmProvider::new_with_sidecar_client(
        crate::provider_runtime::DynamicProviderFacade::new(port.clone()),
        Arc::new(ToolStreamClient),
    );
    let response = provider
        .chat_with_config_id(
            None,
            test_chat_request().with_trace_context(LlmTraceContext {
                project_id: Some(1),
                root_project_id: Some(1),
                workflow: "agent".into(),
                step_key: "model_call".into(),
                generation_run_id: None,
                workflow_run_id: None,
                call_key: Some("run-1".into()),
                input_fingerprint_sha256: None,
                context_plan_json: None,
                context_pack_refs_json: None,
            }),
        )
        .await
        .expect("sidecar tool stream should collect");

    let call = response.message.tool_calls.expect("tool call").remove(0);
    assert_eq!(call.id, "call-1");
    assert_eq!(call.arguments, serde_json::json!({"project_id": 7}));
    assert_eq!(response.usage.map(|usage| usage.total_tokens), Some(5));
    assert_eq!(port.limits.lock().unwrap().len(), 1);
    assert_eq!(*port.statuses.lock().unwrap(), ["completed"]);
}

#[tokio::test]
async fn remote_identity_without_snapshot_never_resolves_server_credentials() {
    let port = Arc::new(TestProviderPort::default());
    let provider = DynamicLlmProvider::new(crate::provider_runtime::DynamicProviderFacade::new(
        port.clone(),
    ));
    let result = crate::runtime_config::request_scope(
        7,
        None,
        provider.chat_with_config_id(None, test_chat_request()),
    )
    .await;
    assert!(matches!(result, Err(AppError::ConfigError(_))));
    assert_eq!(
        port.resolutions.load(std::sync::atomic::Ordering::SeqCst),
        0
    );
}

#[tokio::test]
async fn queue_cancellation_ends_a_sidecar_stream_as_cancelled() {
    use futures::StreamExt;
    let port = Arc::new(TestProviderPort::default());
    let provider = DynamicLlmProvider::new_with_sidecar_client(
        crate::provider_runtime::DynamicProviderFacade::new(port.clone()),
        Arc::new(RecordingLlmClient::new(successful_probe_response())),
    );
    let mut request = test_chat_request();
    request.trace_context = Some(LlmTraceContext {
        project_id: None,
        root_project_id: None,
        workflow: "agent".into(),
        step_key: "model".into(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: None,
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    let mut stream = provider
        .chat_stream_with_config_id(None, request)
        .await
        .unwrap();
    port.cancel.cancel();
    assert!(matches!(
        stream.next().await,
        Some(Err(AppError::Cancelled))
    ));
    assert!(stream.next().await.is_none());
    assert_eq!(*port.statuses.lock().unwrap(), ["cancelled"]);
}

#[test]
fn concurrency_identity_excludes_limit_but_separates_credentials() {
    let model = test_llm_model();
    let first = ResolvedLlmModel::new(model.clone())
        .unwrap()
        .concurrency_limit();
    let mut changed = model;
    changed.max_concurrent = 5;
    let second = ResolvedLlmModel::new(changed.clone())
        .unwrap()
        .concurrency_limit();
    assert_eq!(first.key, second.key);
    assert_eq!(second.max_concurrent, 5);
    changed.api_key = Some("another-test-key".into());
    assert_ne!(
        first.key,
        ResolvedLlmModel::new(changed)
            .unwrap()
            .concurrency_limit()
            .key
    );
    assert!(!first.key.contains("test-key"));
}

#[test]
fn sidecar_business_calls_require_trace_context() {
    let error = require_sidecar_business_trace(None, "LLM sidecar chat 请求")
        .expect_err("business sidecar chat without trace must fail");

    assert!(matches!(error, AppError::InvalidArgument(_)));
    assert!(error.to_string().contains("缺少 trace_context"));
    assert!(error.to_string().contains("project/root project"));
}

#[test]
fn authoritative_trace_root_uses_project_itself_for_root_project() {
    assert_eq!(
        validated_authoritative_trace_root_project_id(42, None, None).expect("root project scope"),
        42
    );
}

#[test]
fn authoritative_trace_root_uses_stored_root_for_derived_project() {
    assert_eq!(
        validated_authoritative_trace_root_project_id(42, None, Some(7))
            .expect("derived project scope"),
        7
    );
    assert_eq!(
        validated_authoritative_trace_root_project_id(42, Some(7), Some(7))
            .expect("matching declared root"),
        7
    );
}

#[test]
fn authoritative_trace_root_rejects_declared_scope_mismatch() {
    let error = validated_authoritative_trace_root_project_id(42, Some(42), Some(7))
        .expect_err("mismatched root must fail");

    assert!(matches!(error, AppError::InvalidArgument(_)));
    assert!(error.to_string().contains("declared_root_project_id=42"));
    assert!(
        error
            .to_string()
            .contains("authoritative_root_project_id=7")
    );
}

#[test]
fn authoritative_trace_root_rejects_invalid_stored_scope() {
    let error = validated_authoritative_trace_root_project_id(42, None, Some(0))
        .expect_err("invalid stored root must fail");

    assert!(matches!(error, AppError::InvalidState { .. }));
}

fn test_llm_model() -> LlmProviderConfig {
    LlmProviderConfig {
        id: 1,
        name: "test-llm".to_string(),
        provider: "openai".to_string(),
        base_url: "https://example.invalid/v1".to_string(),
        api_key: Some("test-key".to_string()),
        model: "test-model".to_string(),
        max_concurrent: 1,
        timeout_secs: 30,
        max_context_tokens: 16_384,
        max_output_tokens: 2_048,
        extra_body_json: None,
    }
}

fn test_chat_request() -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage {
            role: MessageRole::User,
            content: "ping".to_string(),
            tool_calls: None,
            tool_call_id: None,
        }],
        temperature: Some(0.0),
        max_tokens: Some(8),
        response_format: None,
        stream_include_usage: false,
        cache_scope_key: None,
        cache_breakpoints: Vec::new(),
        cache_mode: None,
        cache_system_prefix: false,
        cache_user_prefix_len: None,
        trace_context: None,
        context_manifest_json: None,
        tools: None,
        tool_choice: None,
        stop_sequences: None,
    }
}

fn successful_probe_response() -> a4n_llm_client::LlmProbeResponse {
    a4n_service_protocol::ServiceResponse {
        status: a4n_service_protocol::ServiceStatus::Ok,
        trace_id: Some("trace".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(a4n_llm_sidecar_contract::LlmProbeOutput {
            response_preview: "pong".to_string(),
            usage: None,
            finish_reason: Some("stop".to_string()),
        }),
        error: None,
    }
}
