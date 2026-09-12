use serde_json::json;

use super::{
    ChatChunk, ChatMessage, ChatRequest, ChatResponse, LlmTraceContext, ModelCapabilities,
    UsageInfo, is_truncated_finish_reason, normalize_extra_body_json,
};

#[test]
fn detects_known_truncated_finish_reasons() {
    assert!(is_truncated_finish_reason(Some("length")));
    assert!(is_truncated_finish_reason(Some("max_tokens")));
    assert!(!is_truncated_finish_reason(Some("stop")));
    assert!(!is_truncated_finish_reason(None));
}

#[test]
fn collected_responses_require_a_usable_model_terminal() {
    for reason in [
        None,
        Some(""),
        Some("length"),
        Some("max_tokens"),
        Some("content_filter"),
        Some("unknown"),
    ] {
        let mut accumulator = super::ChatStreamAccumulator::default();
        accumulator
            .push(ChatChunk {
                content: Some("nonempty partial answer".to_string()),
                reasoning_content: None,
                tool_calls: None,
                finish_reason: reason.map(str::to_string),
                index: 0,
                usage: None,
            })
            .unwrap();
        assert!(accumulator.finish().is_err(), "must reject {reason:?}");
    }
    assert!(super::validate_finish_reason(Some("stop")).is_ok());
    assert!(super::validate_finish_reason(Some("tool_calls")).is_ok());
}

#[test]
fn single_response_options_cannot_enable_multiple_choices_or_override_output_budget() {
    assert!(normalize_extra_body_json(Some(json!({"n": 1}))).is_ok());
    for value in [
        json!({"n": 2}),
        json!({"n": 0}),
        json!({"n": "1"}),
        json!({"N": 2}),
        json!({"max_completion_tokens": 99999}),
    ] {
        assert!(normalize_extra_body_json(Some(value)).is_err());
    }
}

#[test]
fn chat_request_serialization_skips_internal_trace_fields() {
    let mut request = ChatRequest::new(vec![ChatMessage::user("hello")], None, None, None)
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
    request.cache_scope_key = Some("a4n:workflow_run:7".to_string());
    request.cache_system_prefix = true;
    request.cache_user_prefix_len = Some(5);

    let value = serde_json::to_value(&request).expect("request serializes");
    assert!(value.get("trace_context").is_none());
    assert_eq!(value["stream_include_usage"], json!(true));
    assert_eq!(value["cache_scope_key"], "a4n:workflow_run:7");
    assert_eq!(value["cache_system_prefix"], true);
    assert_eq!(value["cache_user_prefix_len"], 5);
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
fn extra_body_reject_reserved_provider_fields() {
    let err = normalize_extra_body_json(Some(json!({
        "temperature": 0.1
    })))
    .expect_err("reserved fields must be rejected");

    assert!(err.to_string().contains("temperature"));
}

#[test]
fn extra_body_reject_static_prompt_cache_keys() {
    let err = normalize_extra_body_json(Some(json!({
        "prompt_cache_key": "shared-across-unrelated-runs"
    })))
    .expect_err("cache affinity must be derived per generation chain");

    assert!(err.to_string().contains("prompt_cache_key"));
}

#[test]
fn extra_body_keep_provider_specific_fields() {
    let value = normalize_extra_body_json(Some(json!({
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
        "usage": usage,
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
