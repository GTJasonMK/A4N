use super::super::super::*;
use super::super::support::{manifest_with_pack, pack_ref};
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest};

#[test]
fn context_aware_request_attaches_manifest_without_serializing_it() {
    let mut manifest = manifest_with_pack("生成当前片段");
    manifest.output_contract = Some("只输出 JSON".to_string());
    let request =
        context_aware_system_user_request("system", manifest, Some(0.2), None).expect("request");

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

    let err = context_aware_system_user_request("system", manifest, Some(0.2), None)
        .expect_err("empty context manifest must fail");

    assert!(err.to_string().contains("至少需要一个上下文包"));
}

#[test]
fn manifest_rewrite_rejects_a_non_user_context_message() {
    let manifest = manifest_with_pack("当前任务");
    let mut request = ChatRequest::new(
        vec![
            ChatMessage::system("system"),
            ChatMessage::assistant("assistant"),
        ],
        Some(0.2),
        None,
    );

    let err = replace_request_user_content_from_manifest(&mut request, &manifest, &[])
        .expect_err("complex history must fail");

    assert!(err.to_string().contains("只能重写"));
}

#[test]
fn manifest_rewrite_records_the_stable_user_cache_boundary() {
    let manifest = manifest_with_pack("当前任务");
    let mut request =
        context_aware_system_user_request("system", manifest.clone(), None, None).expect("request");
    let selected = PlannedContextPack {
        pack_ref: pack_ref("chapter_source", COMPRESSION_FULL_RAW, 10),
        stable_prefix: true,
        canonical_content_text: Some("冻结章节上下文".to_string()),
    };

    replace_request_user_content_from_manifest(&mut request, &manifest, &[selected])
        .expect("manifest rewrite");

    let prefix_len = request
        .cache_breakpoints
        .last()
        .expect("stable cache boundary")
        .byte_offset;
    let (prefix, dynamic) = request.messages[1].content.split_at(prefix_len);
    assert!(prefix.contains("冻结章节上下文"));
    assert!(!prefix.contains("当前任务"));
    assert!(dynamic.contains("当前任务"));
}

#[test]
fn manifest_rewrite_keeps_history_and_includes_it_in_the_budget() {
    let manifest = manifest_with_pack("current task");
    let mut request =
        context_aware_system_user_request("system", manifest.clone(), None, None).unwrap();
    request
        .messages
        .push(ChatMessage::user("immutable observation"));
    request
        .add_cache_breakpoint(2, "immutable observation".len())
        .unwrap();
    request.messages.push(ChatMessage::user("latest question"));
    replace_request_user_content_from_manifest(&mut request, &manifest, &[]).unwrap();
    assert_eq!(request.messages[2].content, "immutable observation");
    assert_eq!(request.messages[3].content, "latest question");
    assert_eq!(request.cache_breakpoints.last().unwrap().message_index, 2);
    let dynamic = context_manifest_dynamic_messages(&request, &manifest).unwrap();
    assert_eq!(dynamic.len(), 3);
    assert_eq!(dynamic[1].content, "immutable observation");
}
