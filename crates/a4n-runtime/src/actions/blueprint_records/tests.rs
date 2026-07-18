use serde_json::{Value, json};

use super::{GenerateBlueprintInput, world_setting::required_world_setting_value};

#[test]
fn update_world_setting_rejects_null() {
    let err = required_world_setting_value(&Value::Null)
        .expect_err("null world setting should be rejected");

    assert!(err.to_string().contains("world_setting 不能为 null"));
}

#[test]
fn generate_blueprint_input_rejects_unknown_fields() {
    let err = serde_json::from_value::<GenerateBlueprintInput>(json!({
        "project_id": 7,
        "title": "标题",
        "genre": "类型",
        "target_length": 30,
        "projectId": 7
    }))
    .expect_err("unknown fields must fail instead of being ignored");

    assert!(err.to_string().contains("unknown field"));
}
