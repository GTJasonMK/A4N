//! 章节生成 ChatRequest 合同测试。

use super::build_chapter_generation_request;

#[test]
fn moves_outline_and_context_into_manifest() {
    let request = build_chapter_generation_request(
        42,
        3,
        "写作系统提示词".to_string(),
        "## 当前任务\n第3章：追踪线索",
        Some("上一章结尾：主角发现旧信封"),
    )
    .expect("chapter generation request should build");

    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should be attached");
    let pack_kinds = manifest["packs"]
        .as_array()
        .expect("packs should be array")
        .iter()
        .filter_map(|pack| pack["pack_kind"].as_str())
        .collect::<Vec<_>>();
    assert!(pack_kinds.contains(&"writing_chapter_outline"));
    assert!(pack_kinds.contains(&"writing_context_before"));

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, a4n_llm::provider::MessageRole::User))
        .expect("user message should exist");
    assert!(user_message.content.contains("context_refs"));
    assert!(user_message.content.contains("writing_chapter_outline"));
    assert!(user_message.content.contains("writing_context_before"));
    assert!(!user_message.content.contains("追踪线索"));
    assert!(!user_message.content.contains("旧信封"));

    let trace_context = request
        .trace_context
        .as_ref()
        .expect("trace context should be attached");
    assert_eq!(trace_context.workflow, "chapter_generation");
    assert_eq!(trace_context.step_key, "chapter_3");
    assert_eq!(trace_context.call_key.as_deref(), Some("chapter_3"));
}
