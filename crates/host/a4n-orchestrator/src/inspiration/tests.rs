use a4n_db_models::models::user::message;
use a4n_error::AppError;
use chrono::NaiveDate;

use super::context::optional_trimmed_string;
use super::request::build_inspiration_conversation_messages;
use super::validation::{required_message_content, validate_inspiration_response_json};

fn sample_message(id: i64) -> message::Model {
    message::Model {
        id,
        conversation_id: 1,
        role: "user".to_string(),
        content: format!("消息 {id}"),
        tool_calls: None,
        prompt_tokens: None,
        completion_tokens: None,
        chapter_number: None,
        is_important: false,
        created_at: NaiveDate::from_ymd_opt(2026, 5, 31)
            .unwrap()
            .and_hms_opt(0, 0, id as u32)
            .unwrap(),
    }
}

#[test]
fn optional_trimmed_string_omits_blank_values() {
    assert_eq!(
        optional_trimmed_string("  星潮  "),
        Some("星潮".to_string())
    );
    assert_eq!(optional_trimmed_string(" \n\t "), None);
}

#[test]
fn inspiration_history_is_an_unchanged_prefix_before_the_latest_user_message() {
    let mut assistant = sample_message(4);
    assistant.role = "assistant".to_string();
    assistant.content = "可以从时间循环切入。".to_string();
    let mut user = sample_message(5);
    user.role = "user".to_string();
    user.content = "我想写校园悬疑。".to_string();

    let history = [assistant, user];
    let first = build_inspiration_conversation_messages(&history, "继续深化主角").unwrap();
    let next = build_inspiration_conversation_messages(&history, "调整冲突").unwrap();
    assert_eq!(
        serde_json::to_value(&first[..2]).unwrap(),
        serde_json::to_value(&next[..2]).unwrap()
    );
    assert_eq!(first[0].content, "可以从时间循环切入。");
    assert_eq!(first[1].content, "我想写校园悬疑。");
    assert_eq!(first[2].content, "继续深化主角");
    assert_eq!(next[2].content, "调整冲突");
    assert!(matches!(
        first[0].role,
        a4n_llm_provider_contract::provider::MessageRole::Assistant
    ));
}

#[test]
fn validate_inspiration_response_json_accepts_object() {
    let raw = r#" { "ai_message": "可以从主角误解切入", "next_question": null, "progress_summary": null, "ui_control": {"type": "info_display"}, "conversation_state": {}, "is_complete": false } "#;
    let output = validate_inspiration_response_json(raw).expect("json object should be accepted");

    assert_eq!(output, raw.trim());
}

#[test]
fn validate_inspiration_response_json_rejects_array() {
    let err = validate_inspiration_response_json(r#"[{"reply":"ok"}]"#)
        .expect_err("array must not be accepted as inspiration response");

    assert!(err.to_string().contains("严格 JSON 对象"));
}

#[test]
fn validate_inspiration_response_json_rejects_markdown_fence() {
    let err = validate_inspiration_response_json("```json\n{\"reply\":\"ok\"}\n```")
        .expect_err("markdown fenced json must not be accepted");

    assert!(err.to_string().contains("严格 JSON 对象"));
}

#[test]
fn required_message_content_trims_non_blank_text_and_rejects_blank_text() {
    assert_eq!(
        required_message_content("  关键灵感  ").unwrap(),
        "关键灵感"
    );
    assert!(matches!(
        required_message_content(" \n\t "),
        Err(AppError::InvalidArgument(message)) if message == "消息内容不能为空"
    ));
}
