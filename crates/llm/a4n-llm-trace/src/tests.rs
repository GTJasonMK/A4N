use serde_json::json;

use a4n_llm_context::{
    CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextManifest, LlmContextManifestPack,
    LlmContextManifestPackVariant,
};
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest, UsageInfo};

use super::{
    chat_request_messages_json, chat_request_trace_json, context_aware_chat_request_trace_json,
    llm_usage_summary_text, text_response_summary_json,
};

#[test]
fn formats_usage_with_all_cache_reporting_shapes() {
    assert_eq!(llm_usage_summary_text(None), "模型未返回 token usage");

    let usage = |hit, miss| UsageInfo {
        prompt_tokens: 12,
        completion_tokens: 3,
        total_tokens: 15,
        prompt_cache_hit_tokens: hit,
        prompt_cache_miss_tokens: miss,
        prompt_cache_write_tokens: None,
    };
    assert_eq!(
        llm_usage_summary_text(Some(&usage(Some(3), Some(1)))),
        "Token 15（输入 12 / 输出 3），缓存命中 75.0%（命中 3 / 未命中 1）"
    );
    assert_eq!(
        llm_usage_summary_text(Some(&usage(Some(0), Some(0)))),
        "Token 15（输入 12 / 输出 3），缓存命中 0.0%（命中 0 / 未命中 0）"
    );
    assert_eq!(
        llm_usage_summary_text(Some(&usage(Some(3), None))),
        "Token 15（输入 12 / 输出 3），缓存命中 token 3"
    );
    assert_eq!(
        llm_usage_summary_text(Some(&usage(None, Some(4)))),
        "Token 15（输入 12 / 输出 3），缓存未命中 token 4"
    );
    assert_eq!(
        llm_usage_summary_text(Some(&usage(None, None))),
        "Token 15（输入 12 / 输出 3），缓存数据未返回"
    );
}

#[test]
fn projects_provider_request_and_messages_without_internal_context() {
    let mut request = ChatRequest::new(
        vec![ChatMessage::system("system"), ChatMessage::user("user")],
        Some(0.2),
        Some(512),
    );
    request.context_manifest_json = Some(json!({"private": true}));

    let messages = chat_request_messages_json(&request).expect("messages should serialize");
    let trace = chat_request_trace_json(&request).expect("request should serialize");

    assert_eq!(messages.as_array().map(Vec::len), Some(2));
    assert_eq!(trace.get("messages"), Some(&messages));
    let temperature = trace["temperature"]
        .as_f64()
        .expect("temperature should serialize as a number");
    assert!((temperature - 0.2).abs() < 1e-6);
    assert!(trace.get("context_manifest_json").is_none());
    assert!(trace.get("trace_context").is_none());
}

#[test]
fn projects_context_manifest_summary_and_trace_context_for_audit() {
    let manifest = LlmContextManifest {
        schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
        packs: vec![LlmContextManifestPack {
            scope_kind: "project".to_string(),
            pack_kind: "chapter".to_string(),
            source_refs_json: Some(json!({"chapter_id": 7})),
            config_hash: "config-hash".to_string(),
            required: true,
            stable_prefix: false,
            variants: vec![LlmContextManifestPackVariant {
                compression_level: "full_raw".to_string(),
                canonical_content_json: json!({"text": "你好 world"}),
            }],
        }],
        dynamic_user_content: "继续写作".to_string(),
        output_contract: Some("返回 JSON".to_string()),
    };
    let request = ChatRequest::system_user("system", "user", None, None)
        .with_context_manifest_json(serde_json::to_value(manifest).expect("manifest serializes"))
        .with_trace_context(a4n_llm_provider_contract::provider::LlmTraceContext {
            project_id: Some(7),
            root_project_id: Some(3),
            workflow: "chapter_generation".to_string(),
            step_key: "draft".to_string(),
            generation_run_id: Some(11),
            workflow_run_id: Some(13),
            call_key: Some("chapter:7:draft".to_string()),
            input_fingerprint_sha256: Some("fingerprint".to_string()),
            context_plan_json: Some(json!({"selected": 1})),
            context_pack_refs_json: Some(json!([{"pack_kind": "chapter"}])),
        });

    let trace = context_aware_chat_request_trace_json(&request).expect("trace should project");

    assert!(trace.get("context_manifest_json").is_none());
    assert_eq!(trace["context_manifest_summary"]["schema_version"], 1);
    assert_eq!(
        trace["context_manifest_summary"]["packs"][0]["scope_kind"],
        "project"
    );
    assert_eq!(
        trace["context_manifest_summary"]["packs"][0]["variants"][0]["compression_level"],
        "full_raw"
    );
    assert_eq!(trace["trace_context"]["project_id"], 7);
    assert_eq!(trace["trace_context"]["root_project_id"], 3);
    assert_eq!(trace["trace_context"]["workflow"], "chapter_generation");
    assert_eq!(trace["trace_context"]["context_plan"]["selected"], 1);
}

#[test]
fn summarizes_text_and_merges_object_extras() {
    let summary = text_response_summary_json(
        "abc",
        Some("stop"),
        json!({"model": "test", "content_chars": 99}),
    );

    assert_eq!(summary["content_chars"], 99);
    assert_eq!(
        summary["content_sha256"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
    assert_eq!(summary["content_preview"], "abc");
    assert_eq!(summary["finish_reason"], "stop");
    assert_eq!(summary["model"], "test");
}
