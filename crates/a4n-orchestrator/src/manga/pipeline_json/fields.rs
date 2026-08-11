use a4n_core::error::AppError;
use serde_json::Value;

pub(crate) fn required_object_field(
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

pub(crate) fn required_array_field(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<Value, AppError> {
    let value = required_field(json, key, context)?;
    if !value.is_array() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是数组"
        )));
    }
    Ok(value.clone())
}

fn required_field<'a>(json: &'a Value, key: &str, context: &str) -> Result<&'a Value, AppError> {
    json.get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 缺少必需字段 {key}")))
}
