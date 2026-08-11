use crate::request::json_user_message_payload;
use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::build_chapter_generation_user_message;

fn parse_user_payload(content: &str) -> Result<Value, AppError> {
    let payload = json_user_message_payload(content).ok_or_else(|| {
        AppError::InvalidArgument("章节生成 user message 缺少结构化 JSON 输入前缀".to_string())
    })?;
    serde_json::from_str(payload).map_err(|e| {
        AppError::InvalidArgument(format!("章节生成 user message 不是合法 JSON payload: {e}"))
    })
}

#[test]
fn builds_scene_state_refs_for_plain_context() {
    let message = build_chapter_generation_user_message("## 当前任务\n第2章", Some("上一章结尾"))
        .expect("valid generation outline");
    let payload = parse_user_payload(&message).expect("payload");
    assert_eq!(payload["task"]["name"], json!("chapter_generation"));
    assert_eq!(
        payload["task"]["instruction"],
        json!("根据当前章节任务、章节连续性上下文和参考上下文生成完整章节正文")
    );
    assert_eq!(
        payload["context_refs"]["chapter_outline"],
        json!("稳定上下文包 writing_chapter_outline")
    );
    assert_eq!(
        payload["context_refs"]["context_before"],
        json!("稳定上下文包 writing_context_before")
    );
    assert_eq!(payload["output_contract"]["format"], json!("plain_text"));
    assert!(payload["output_contract"].get("forbidden_fields").is_none());
    assert!(!message.contains("full_content"));
    assert!(!message.contains("上一章结尾"));
}

#[test]
fn preserves_structured_context_sections_as_refs() {
    let message = build_chapter_generation_user_message(
        "## 当前任务\n第3章",
        Some("## 前文摘要\n第1章：开端\n\n## 上一章结尾\n尾段"),
    )
    .expect("valid generation outline");
    let payload = parse_user_payload(&message).expect("payload");
    assert_eq!(
        payload["context_refs"]["chapter_outline"],
        json!("稳定上下文包 writing_chapter_outline")
    );
    assert_eq!(
        payload["context_refs"]["context_before"],
        json!("稳定上下文包 writing_context_before")
    );
    assert!(!message.contains("第1章：开端"));
    assert!(!message.contains("上一章结尾"));
}

#[test]
fn omits_missing_context_section() {
    let message = build_chapter_generation_user_message("## 当前任务\n第1章", None)
        .expect("valid generation outline");
    let payload = parse_user_payload(&message).expect("payload");
    assert_eq!(
        payload["context_refs"]["chapter_outline"],
        json!("稳定上下文包 writing_chapter_outline")
    );
    assert!(payload["context_refs"]["context_before"].is_null());
}

#[test]
fn rejects_missing_outline() {
    let err = build_chapter_generation_user_message("  ", Some("上一章结尾"))
        .expect_err("blank generation outline should fail");
    assert!(matches!(err, AppError::InvalidArgument(_)));
    assert!(err.to_string().contains("缺少当前任务"));
}
