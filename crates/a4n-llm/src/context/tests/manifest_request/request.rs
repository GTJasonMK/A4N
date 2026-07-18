use super::super::super::*;
use super::super::support::manifest_with_pack;
use crate::provider::{ChatMessage, ChatRequest};

#[test]
fn context_aware_request_attaches_manifest_without_serializing_it() {
    let mut manifest = manifest_with_pack("生成当前片段");
    manifest.output_contract = Some("只输出 JSON".to_string());
    let request = context_aware_system_user_request(
        "system",
        manifest,
        Some(0.2),
        None,
        Some("media_prompt".to_string()),
    )
    .expect("request");

    assert!(request.context_manifest_json.is_some());
    assert_eq!(request.messages.len(), 2);
    assert!(request.messages[1].content.contains("[当前任务]"));
    assert!(request.messages[1].content.contains("[输出契约]"));
    let value = serde_json::to_value(&request).expect("serialize request");
    assert!(value.get("context_manifest_json").is_none());
}

#[test]
fn context_aware_request_rejects_empty_manifest_packs() {
    let manifest = LlmContextManifest {
        schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
        packs: Vec::new(),
        dynamic_user_content: "生成当前片段".to_string(),
        output_contract: None,
    };

    let err = context_aware_system_user_request(
        "system",
        manifest,
        Some(0.2),
        None,
        Some("media_prompt".to_string()),
    )
    .expect_err("empty context manifest must fail");

    assert!(err.to_string().contains("至少需要一个上下文包"));
}

#[test]
fn manifest_rewrite_rejects_complex_chat_history() {
    let manifest = manifest_with_pack("当前任务");
    let mut request = ChatRequest::new(
        vec![
            ChatMessage::system("system"),
            ChatMessage::user("first"),
            ChatMessage::assistant("assistant"),
        ],
        Some(0.2),
        None,
        None,
    );

    let err = replace_request_user_content_from_manifest(&mut request, &manifest, &[])
        .expect_err("complex history must fail");

    assert!(err.to_string().contains("只能重写"));
}
