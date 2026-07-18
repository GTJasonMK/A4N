use super::contracts::{ConverseInput, INSPIRATION_VARIABLES_ERROR};
use super::rules::{inspiration_stream_task_id, validate_history_limit};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use serde_json::json;

#[test]
fn inspiration_stream_task_id_generates_id_when_absent() {
    let task_id = inspiration_stream_task_id(9, None).expect("generated task id");
    assert!(task_id.starts_with("inspiration-9-"));
}

#[test]
fn inspiration_stream_task_id_accepts_trimmed_safe_id() {
    let task_id = inspiration_stream_task_id(9, Some("  inspiration:9_alpha-1.test  ".to_string()))
        .expect("provided task id");
    assert_eq!(task_id, "inspiration:9_alpha-1.test");
}

#[test]
fn inspiration_stream_task_id_rejects_invalid_provided_id() {
    assert!(inspiration_stream_task_id(9, Some("   ".to_string())).is_err());
    assert!(inspiration_stream_task_id(9, Some("灵感-9".to_string())).is_err());
    assert!(inspiration_stream_task_id(9, Some("a".repeat(161))).is_err());
}

#[test]
fn history_limit_must_be_positive() {
    assert!(validate_history_limit(1).is_ok());
    let error = validate_history_limit(0).expect_err("zero limit should fail");
    assert!(error.to_string().contains("limit 必须大于 0"));
}

#[test]
fn variables_must_be_object_when_present() {
    let variables = Some(json!(["tone"]));
    let error = parse_template_variables_app_error_from_ref_with_message(
        variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )
    .expect_err("array variables should fail");
    assert!(error.to_string().contains("variables 必须是 JSON 对象"));
}

#[test]
fn converse_input_rejects_unknown_fields() {
    let error = serde_json::from_value::<ConverseInput>(json!({
        "project_id": 7,
        "user_input": "继续",
        "projectId": 7
    }))
    .expect_err("unknown fields must fail instead of being ignored");
    assert!(error.to_string().contains("unknown field"));
}
