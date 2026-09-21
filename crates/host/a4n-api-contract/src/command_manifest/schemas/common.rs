use super::builders::*;
use serde_json::{Value, json};

pub(crate) fn args_config_id() -> Value {
    json!({
        "type": "object",
        "properties": { "configId": { "type": "integer", "minimum": 1 } },
        "required": ["configId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_project_id() -> Value {
    json!({
        "type": "object",
        "properties": { "projectId": { "type": "integer", "minimum": 1 } },
        "required": ["projectId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_user_id() -> Value {
    json!({
        "type": "object",
        "properties": { "userId": { "type": "integer", "minimum": 1 } },
        "required": ["userId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_task_id() -> Value {
    json!({
        "type": "object",
        "properties": { "taskId": { "type": "string", "minLength": 1 } },
        "required": ["taskId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_invite_id() -> Value {
    json!({
        "type": "object",
        "properties": { "inviteId": { "type": "integer", "minimum": 1 } },
        "required": ["inviteId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn args_member_user_id() -> Value {
    json!({
        "type": "object",
        "properties": { "memberUserId": { "type": "integer", "minimum": 1 } },
        "required": ["memberUserId"],
        "additionalProperties": false,
        "x-a4n-schema-quality": "typed",
    })
}
pub(crate) fn result_json_value() -> Value {
    let mut schema = raw_schema::<Value>();
    schema
        .as_object_mut()
        .expect("serde_json::Value schema must be an object")
        .insert(
            "x-a4n-schema-quality".into(),
            Value::String("json-value".into()),
        );
    schema
}
pub(crate) fn result_optional_json_value() -> Value {
    typed_schema::<Option<Value>>()
}
pub(crate) fn result_unit() -> Value {
    typed_schema::<()>()
}
pub(crate) fn result_string() -> Value {
    typed_schema::<String>()
}
