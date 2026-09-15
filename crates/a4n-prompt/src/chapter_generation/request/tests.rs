//! 章节生成 ChatRequest 合同测试。

use super::build_chapter_generation_request;
use a4n_llm_context::preview_context_aware_chat_request;
use a4n_llm_provider_contract::provider::MessageRole;

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
        .find(|message| matches!(message.role, MessageRole::User))
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

#[test]
fn minimal_required_packs_keep_actual_outline_and_continuity_constraints() {
    let outline = format!(
        "## 当前任务\n关键章纲起点\n{}\n章纲末尾约束",
        "章".repeat(12_000)
    );
    let context = format!(
        "## 后续已写正文约束\n后续落点必须保留\n{}\n## 上一章结尾\n上一章结尾约束",
        "文".repeat(16_000)
    );
    let mut request = build_chapter_generation_request(
        42,
        5,
        "写作系统提示词".to_string(),
        &outline,
        Some(&context),
    )
    .expect("chapter generation request should build");

    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should exist");
    for pack in manifest["packs"]
        .as_array()
        .expect("manifest packs should be an array")
        .iter()
        .filter(|pack| {
            matches!(
                pack["pack_kind"].as_str(),
                Some("writing_chapter_outline" | "writing_context_before")
            )
        })
    {
        let text_key = if pack["pack_kind"] == "writing_chapter_outline" {
            "chapter_outline"
        } else {
            "context_before"
        };
        let lengths = pack["variants"]
            .as_array()
            .expect("pack variants should be an array")
            .iter()
            .map(|variant| {
                variant["canonical_content_json"][text_key]
                    .as_str()
                    .expect("required pack variant should retain real text")
                    .chars()
                    .count()
            })
            .collect::<Vec<_>>();
        assert!(lengths[0] > lengths[1]);
        assert!(lengths[1] > lengths[2]);
    }

    let preview = preview_context_aware_chat_request(&mut request, 10_000, 2_000)
        .expect("context planning should succeed")
        .expect("context manifest should be planned");
    let selected_levels = preview
        .pack_refs_json
        .as_array()
        .expect("selected pack refs should be an array")
        .iter()
        .filter(|pack| {
            matches!(
                pack["pack_kind"].as_str(),
                Some("writing_chapter_outline" | "writing_context_before")
            )
        })
        .filter_map(|pack| pack["compression_level"].as_str())
        .collect::<Vec<_>>();
    assert_eq!(selected_levels, vec!["minimal", "minimal"]);

    let user_message = request
        .messages
        .iter()
        .find(|message| matches!(message.role, MessageRole::User))
        .expect("rewritten user message should exist");
    assert!(user_message.content.contains("关键章纲起点"));
    assert!(user_message.content.contains("章纲末尾约束"));
    assert!(user_message.content.contains("## 后续已写正文约束"));
    assert!(user_message.content.contains("后续落点必须保留"));
    assert!(user_message.content.contains("上一章结尾约束"));
    assert!(
        user_message
            .content
            .contains("中间内容已按模型上下文预算压缩")
    );
}
