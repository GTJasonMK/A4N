use super::contracts::{INSPIRATION_VARIABLES_ERROR, PreviewInspirationRequestInput};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use a4n_orchestrator::inspiration::validate_inspiration_history_limit;
use serde_json::json;

#[test]
fn history_limit_must_be_positive() {
    assert!(validate_inspiration_history_limit(1).is_ok());
    let error = validate_inspiration_history_limit(0).expect_err("zero limit should fail");
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
fn preview_input_rejects_unknown_fields() {
    let error = serde_json::from_value::<PreviewInspirationRequestInput>(json!({
        "project_id": 7,
        "user_input": "继续",
        "projectId": 7
    }))
    .expect_err("unknown fields must fail instead of being ignored");
    assert!(error.to_string().contains("unknown field"));
}
