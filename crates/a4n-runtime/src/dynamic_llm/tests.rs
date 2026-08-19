use std::sync::Arc;

use a4n_test_support::capability::RecordingLlmClient;
use a4n_core::config::app_config::DEFAULT_LLM_MAX_OUTPUT_TOKENS;
use a4n_core::error::AppError;
use a4n_core::models::ai::llm_config;
use a4n_llm::provider::{ChatMessage, ChatRequest, MessageRole};

use crate::dynamic_llm::trace::validated_authoritative_trace_root_project_id;

use super::budget::ensure_max_tokens_within_model_cap;
use super::config::{probe_llm_via_sidecar, require_sidecar_business_trace};
use super::contracts::ResolvedLlmModel;

#[test]
fn max_tokens_cap_accepts_missing_or_within_limit() {
    assert!(
        ensure_max_tokens_within_model_cap(
            None,
            None,
            DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            "deepseek-chat"
        )
        .is_ok()
    );
    assert!(
        ensure_max_tokens_within_model_cap(
            Some(DEFAULT_LLM_MAX_OUTPUT_TOKENS),
            Some("media_prompt"),
            DEFAULT_LLM_MAX_OUTPUT_TOKENS,
            "deepseek-chat",
        )
        .is_ok()
    );
}

#[test]
fn max_tokens_cap_rejects_profile_above_model_limit() {
    let err = ensure_max_tokens_within_model_cap(
        Some(12_288),
        Some("character_sync"),
        DEFAULT_LLM_MAX_OUTPUT_TOKENS,
        "deepseek-chat",
    )
    .expect_err("profile output limit must not exceed model capability");

    assert!(matches!(err, AppError::ConfigError(_)));
    assert!(err.to_string().contains("profile=character_sync"));
    assert!(err.to_string().contains("model_max_output_tokens=8192"));
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

fn test_llm_model() -> llm_config::Model {
    let now = chrono::NaiveDate::from_ymd_opt(2026, 1, 1)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap();
    llm_config::Model {
        id: 1,
        name: "test-llm".to_string(),
        provider: "openai".to_string(),
        base_url: "https://example.invalid/v1".to_string(),
        api_key_encrypted: Some("test-key".to_string()),
        model: "test-model".to_string(),
        max_concurrent: 1,
        timeout_secs: 30,
        max_context_tokens: 16_384,
        max_output_tokens: 2_048,
        extra_body_json: None,
        is_default: true,
        created_at: now,
        updated_at: now,
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
        max_tokens_profile: None,
        response_format: None,
        stream_include_usage: false,
        cache_scope_key: None,
        cache_system_prefix: false,
        cache_user_prefix_len: None,
        trace_context: None,
        context_manifest_json: None,
        tools: None,
        tool_choice: None,
        stop_sequences: None,
    }
}

fn successful_probe_response() -> a4n_capabilities::JsonResponse {
    a4n_service_protocol::ServiceResponse {
        status: a4n_service_protocol::ServiceStatus::Ok,
        trace_id: Some("trace".to_string()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(serde_json::json!({
            "response_preview": "pong",
            "usage": null,
            "finish_reason": "stop"
        })),
        error: None,
    }
}
