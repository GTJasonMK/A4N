use super::{reject_unknown_keys, required_field};
use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::blueprint::json) fn required_object_with_keys(
    json: &Value,
    key: &str,
    context: &str,
    required_keys: &[&str],
) -> Result<Value, AppError> {
    let value = required_field(json, key, context)?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是 JSON 对象"
        )));
    }
    let object = value
        .as_object()
        .expect("value.is_object checked immediately above");
    let child_context = format!("{context}.{key}");
    reject_unknown_keys(object, required_keys, &child_context)?;
    for required_key in required_keys {
        required_non_empty_value(value, required_key, &child_context)?;
    }
    Ok(value.clone())
}

pub(in crate::blueprint::json) fn required_non_empty_value(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<(), AppError> {
    let value = required_field(json, key, context)?;
    let is_empty = match value {
        Value::Null => true,
        Value::String(text) => text.trim().is_empty(),
        Value::Array(items) => items.is_empty(),
        Value::Object(items) => items.is_empty(),
        Value::Bool(_) | Value::Number(_) => false,
    };
    if is_empty {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 不能为空"
        )));
    }
    Ok(())
}
