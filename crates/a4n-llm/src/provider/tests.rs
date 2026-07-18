use serde_json::json;

use super::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmTraceContext, ModelCapabilities,
    UsageInfo, is_truncated_finish_reason, normalize_request_options_json,
};

#[test]
fn detects_known_truncated_finish_reasons() {
    assert!(is_truncated_finish_reason(Some("length")));
    assert!(is_truncated_finish_reason(Some("max_tokens")));
    assert!(!is_truncated_finish_reason(Some("stop")));
    assert!(!is_truncated_finish_reason(None));
}

#[test]
fn chat_request_serialization_skips_internal_trace_fields() {
    let request = ChatRequest::new(vec![ChatMessage::user("hello")], None, None, None)
        .with_stream_usage()
        .with_trace_context(LlmTraceContext {
            project_id: Some(1),
            root_project_id: Some(1),
            workflow: "test".to_string(),
            step_key: "step".to_string(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: None,
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });

    let value = serde_json::to_value(&request).expect("request serializes");
    assert!(value.get("trace_context").is_none());
    assert_eq!(value["stream_include_usage"], json!(true));
    assert!(value.get("context_manifest_json").is_none());
}

#[test]
fn chat_request_serialization_omits_default_stream_usage() {
    let request = ChatRequest::new(vec![ChatMessage::user("hello")], None, None, None);

    let value = serde_json::to_value(&request).expect("request serializes");
    assert!(value.get("stream_include_usage").is_none());
}

#[test]
fn chat_request_serializes_json_response_format() {
    let request = ChatRequest::new(vec![ChatMessage::user("json")], None, None, None)
        .with_json_response_format();

    let value = serde_json::to_value(&request).expect("request serializes");
    assert_eq!(value["response_format"]["type"], "json_object");
}

#[test]
fn request_options_reject_reserved_provider_fields() {
    let err = normalize_request_options_json(Some(json!({
        "temperature": 0.1
    })))
    .expect_err("reserved fields must be rejected");

    assert!(err.to_string().contains("temperature"));
}

#[test]
fn request_options_keep_provider_specific_fields() {
    let value = normalize_request_options_json(Some(json!({
        "extra_body": {
            "reasoning_effort": "low"
        }
    })))
    .expect("provider-specific fields should be accepted");

    assert_eq!(
        value,
        Some(json!({"extra_body": {"reasoning_effort": "low"}}))
    );
}

#[test]
fn nested_llm_contracts_reject_unknown_fields() {
    let error = serde_json::from_value::<ModelCapabilities>(json!({
        "function_calling": false,
        "vision": false,
        "max_context_tokens": 8192,
        "max_output_tokens": 2048,
        "legacy_capability": true
    }))
    .expect_err("model capabilities must reject unknown fields");
    assert!(error.to_string().contains("legacy_capability"));

    let error = serde_json::from_value::<LlmTraceContext>(json!({
        "project_id": 1,
        "root_project_id": 1,
        "workflow": "test",
        "step_key": "step",
        "legacy_trace": true
    }))
    .expect_err("trace context must reject unknown fields");
    assert!(error.to_string().contains("legacy_trace"));

    let usage = json!({
        "prompt_tokens": 1,
        "completion_tokens": 2,
        "total_tokens": 3,
        "prompt_cache_hit_tokens": null,
        "prompt_cache_miss_tokens": null
    });
    let error = serde_json::from_value::<UsageInfo>(json!({
        "prompt_tokens": 1,
        "completion_tokens": 2,
        "total_tokens": 3,
        "prompt_cache_hit_tokens": null,
        "prompt_cache_miss_tokens": null,
        "legacy_usage": 4
    }))
    .expect_err("usage must reject unknown fields");
    assert!(error.to_string().contains("legacy_usage"));

    let error = serde_json::from_value::<ChatChunk>(json!({
        "content": "delta",
        "reasoning_content": null,
        "tool_calls": null,
        "finish_reason": null,
        "index": 0,
        "usage": usage.clone(),
        "legacy_chunk": true
    }))
    .expect_err("chat chunk must reject unknown fields");
    assert!(error.to_string().contains("legacy_chunk"));

    let error = serde_json::from_value::<ChatResponse>(json!({
        "message": {"role": "assistant", "content": "done"},
        "usage": usage,
        "finish_reason": "stop",
        "legacy_response": true
    }))
    .expect_err("chat response must reject unknown fields");
    assert!(error.to_string().contains("legacy_response"));
}
