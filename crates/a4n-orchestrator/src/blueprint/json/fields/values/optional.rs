use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::blueprint::json) fn optional_string(
    json: &Value,
    key: &str,
) -> Result<Option<String>, AppError> {
    let Some(value) = json.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let text = value
        .as_str()
        .ok_or_else(|| AppError::InvalidArgument(format!("{key} 必须是字符串")))?
        .trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!("{key} 不能为空")));
    }
    Ok(Some(text.to_string()))
}
