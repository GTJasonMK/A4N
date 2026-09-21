use a4n_db_models::models::user::message;
use a4n_error::AppError;
use chrono::NaiveDate;

use super::context::optional_trimmed_string;
use super::request::build_inspiration_dynamic_user_content;
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
fn inspiration_dynamic_user_content_keeps_history_dynamic_and_context_referenced() {
    let mut assistant = sample_message(4);
    assistant.role = "assistant".to_string();
    assistant.content = "可以从时间循环切入。".to_string();
    let mut user = sample_message(5);
    user.role = "user".to_string();
    user.content = "我想写校园悬疑。".to_string();

    let content = build_inspiration_dynamic_user_content(&[assistant, user], "继续深化主角")
        .expect("dynamic inspiration input should serialize");

    assert!(content.contains("inspiration_project_context"));
    assert!(content.contains("可以从时间循环切入。"));
    assert!(content.contains("我想写校园悬疑。"));
    assert!(content.contains("继续深化主角"));
    assert!(content.contains("strict_json_object"));
    assert!(!content.contains("project_context_json"));
}

#[test]
fn validate_inspiration_response_json_accepts_object() {
    let output = validate_inspiration_response_json(
        r#" { "reply": "可以从主角误解切入", "is_complete": false } "#,
    )
    .expect("json object should be accepted");

    assert_eq!(
        output,
        r#"{ "reply": "可以从主角误解切入", "is_complete": false }"#
    );
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
