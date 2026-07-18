use a4n_core::error::AppError;
use serde_json::Value;

fn optional_strength(json: &Value, key: &str, context: &str) -> Result<Option<i32>, AppError> {
    let Some(value) = json.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let raw = value
        .as_i64()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是整数")))?;
    let value = i32::try_from(raw)
        .map_err(|_| AppError::InvalidArgument(format!("{context}.{key} 超出 i32 可表示范围")))?;
    if !(1..=10).contains(&value) {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须在 1 到 10 之间"
        )));
    }
    Ok(Some(value))
}

pub(in crate::blueprint::json) fn required_strength(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    optional_strength(json, key, context)?
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 缺少必填字段 {key}")))
}
