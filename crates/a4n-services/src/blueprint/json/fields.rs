use a4n_core::error::AppError;
use serde_json::{Map, Value};

use super::{MAX_TOTAL_CHAPTERS, MIN_TOTAL_CHAPTERS};

pub(super) fn reject_keys(
    object: &Map<String, Value>,
    keys: &[&str],
    context: &str,
) -> Result<(), AppError> {
    for key in keys {
        if object.contains_key(*key) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 包含旧字段 {key}，请使用当前蓝图 JSON schema"
            )));
        }
    }
    Ok(())
}

pub(super) fn reject_unknown_keys(
    object: &Map<String, Value>,
    allowed: &[&str],
    context: &str,
) -> Result<(), AppError> {
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{context} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

pub(super) fn validate_chapter_count(value: i32, context: &str) -> Result<(), AppError> {
    if (MIN_TOTAL_CHAPTERS..=MAX_TOTAL_CHAPTERS).contains(&value) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{context} 必须在 {MIN_TOTAL_CHAPTERS} 到 {MAX_TOTAL_CHAPTERS} 之间"
        )))
    }
}

pub(super) fn validate_enum(
    value: &str,
    key: &str,
    context: &str,
    allowed: &[&str],
) -> Result<(), AppError> {
    if allowed.contains(&value) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是以下值之一：{}",
            allowed.join(", ")
        )))
    }
}

pub(super) fn required_field<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Value, AppError> {
    json.get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 缺少必填字段 {key}")))
}

pub(super) fn required_string(json: &Value, key: &str, context: &str) -> Result<String, AppError> {
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

pub(super) fn optional_string(json: &Value, key: &str) -> Result<Option<String>, AppError> {
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

pub(super) fn required_i32(json: &Value, key: &str, context: &str) -> Result<i32, AppError> {
    let raw = required_field(json, key, context)?
        .as_i64()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是整数")))?;
    i32::try_from(raw)
        .map_err(|_| AppError::InvalidArgument(format!("{context}.{key} 超出 i32 可表示范围")))
}

pub(super) fn required_bool(json: &Value, key: &str, context: &str) -> Result<bool, AppError> {
    required_field(json, key, context)?
        .as_bool()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是布尔值")))
}

pub(super) fn required_array<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, AppError> {
    required_field(json, key, context)?
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是数组")))
}

pub(super) fn required_object_clone(
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

pub(super) fn required_object_with_keys(
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

pub(super) fn required_non_empty_value(
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

pub(super) fn optional_strength(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<Option<i32>, AppError> {
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

pub(super) fn required_strength(json: &Value, key: &str, context: &str) -> Result<i32, AppError> {
    optional_strength(json, key, context)?
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 缺少必填字段 {key}")))
}
