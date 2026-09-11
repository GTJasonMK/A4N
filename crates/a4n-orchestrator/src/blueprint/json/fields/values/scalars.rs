use a4n_error::AppError;
use serde_json::Value;

use super::required_field;

pub(in crate::blueprint::json) fn required_string(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<String, AppError> {
    let text = required_field(json, key, context)?
        .as_str()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是字符串")))?
        .trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 不能为空"
        )));
    }
    Ok(text.to_string())
}

pub(in crate::blueprint::json) fn required_i32(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    let raw = required_field(json, key, context)?
        .as_i64()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是整数")))?;
    i32::try_from(raw)
        .map_err(|_| AppError::InvalidArgument(format!("{context}.{key} 超出 i32 可表示范围")))
}

pub(in crate::blueprint::json) fn required_bool(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<bool, AppError> {
    required_field(json, key, context)?
        .as_bool()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是布尔值")))
}
