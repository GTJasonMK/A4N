use a4n_error::AppError;
use serde_json::Value;

use super::required_field;

pub(in crate::blueprint::json) fn required_array<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, AppError> {
    required_field(json, key, context)?
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是数组")))
}

pub(in crate::blueprint::json) fn required_object_clone(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<Value, AppError> {
    let value = required_field(json, key, context)?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是 JSON 对象"
        )));
    }
    Ok(value.clone())
}
