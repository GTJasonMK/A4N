use a4n_core::error::AppError;
use serde_json::Value;

pub(in crate::outline) fn required_string(json: &Value, key: &str) -> Result<String, AppError> {
    let value = json
        .get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲结果缺少必需字段: {key}")))?;
    let text = value
        .as_str()
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲字段 {key} 必须是字符串")))?
        .trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 不能为空"
        )));
    }
    Ok(text.to_string())
}

pub(in crate::outline) fn required_i32(
    json: &Value,
    key: &str,
    expected: i32,
) -> Result<i32, AppError> {
    let raw = json
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲字段 {key} 必须是整数")))?;
    if raw != expected as i64 {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 必须等于 {expected}"
        )));
    }
    Ok(expected)
}

pub(in crate::outline) fn required_array(json: &Value, key: &str) -> Result<Value, AppError> {
    let value = json
        .get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲结果缺少必需字段: {key}")))?;
    if !value.is_array() {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 必须是数组"
        )));
    }
    Ok(value.clone())
}

pub(in crate::outline) fn optional_array(
    json: &Value,
    key: &str,
) -> Result<Option<Value>, AppError> {
    let Some(value) = json.get(key) else {
        return Ok(None);
    };
    if !value.is_array() {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 必须是数组"
        )));
    }
    Ok(Some(value.clone()))
}

pub(in crate::outline) fn required_object(json: &Value, key: &str) -> Result<Value, AppError> {
    let value = json
        .get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲结果缺少必需字段: {key}")))?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 必须是对象"
        )));
    }
    Ok(value.clone())
}

pub(in crate::outline) fn optional_non_null(json: &Value, key: &str) -> Option<Value> {
    json.get(key).filter(|value| !value.is_null()).cloned()
}

pub(in crate::outline) fn optional_i32(json: &Value, key: &str) -> Result<Option<i32>, AppError> {
    let Some(value) = json.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let raw = value
        .as_i64()
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲字段 {key} 必须是整数")))?;
    i32::try_from(raw)
        .map(Some)
        .map_err(|_| AppError::InvalidArgument(format!("大纲字段 {key} 超出 i32 范围")))
}

pub(in crate::outline) fn optional_positive_i32(
    json: &Value,
    key: &str,
) -> Result<Option<i32>, AppError> {
    let Some(value) = optional_i32(json, key)? else {
        return Ok(None);
    };
    if value <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 必须大于 0"
        )));
    }
    Ok(Some(value))
}

pub(in crate::outline) fn optional_f32(json: &Value, key: &str) -> Result<Option<f32>, AppError> {
    let Some(value) = json.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let raw = value
        .as_f64()
        .ok_or_else(|| AppError::InvalidArgument(format!("大纲字段 {key} 必须是数字")))?;
    if !raw.is_finite() || raw < f32::MIN as f64 || raw > f32::MAX as f64 {
        return Err(AppError::InvalidArgument(format!(
            "大纲字段 {key} 超出 f32 范围"
        )));
    }
    Ok(Some(raw as f32))
}
