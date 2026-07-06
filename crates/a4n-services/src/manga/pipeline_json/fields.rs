use a4n_core::error::AppError;
use serde_json::Value;

pub(crate) fn parse_json_object(raw: &str, context: &str) -> Result<Value, AppError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回空内容，必须返回 JSON 对象"
        )));
    }
    if !raw.starts_with('{') {
        let preview = raw.chars().take(80).collect::<String>();
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回内容不是 JSON 对象，开头为：{preview}"
        )));
    }

    let value: Value = serde_json::from_str(raw).map_err(|e| {
        if e.is_eof() {
            return AppError::InvalidArgument(format!(
                "{context} 返回了不完整 JSON，通常表示模型输出被截断: {e}"
            ));
        }
        AppError::InvalidArgument(format!("{context} 必须返回可解析的 JSON 对象: {e}"))
    })?;
    require_object_value(value, context)
}

pub(crate) fn require_object_value(value: Value, context: &str) -> Result<Value, AppError> {
    if !value.is_object() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须是 JSON 对象"
        )));
    }
    Ok(value)
}

pub(crate) fn reject_unknown_fields(
    json: &Value,
    allowed: &[&str],
    context: &str,
) -> Result<(), AppError> {
    let object = json
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 必须是 JSON 对象")))?;
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

pub(crate) fn required_positive_i32_field(
    json: &Value,
    key: &str,
    context: &str,
) -> Result<i32, AppError> {
    let raw = required_field(json, key, context)?
        .as_i64()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是整数")))?;
    if raw <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 必须是正整数"
        )));
    }
    i32::try_from(raw)
        .map_err(|_| AppError::InvalidArgument(format!("{context}.{key} 超出 i32 可表示范围")))
}

pub(crate) fn required_non_empty_string_field<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a str, AppError> {
    let text = required_field(json, key, context)?
        .as_str()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是字符串")))?
        .trim();
    if text.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.{key} 不能为空"
        )));
    }
    Ok(text)
}

pub(crate) fn optional_string_field<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<Option<&'a str>, AppError> {
    let Some(value) = json.get(key) else {
        return Ok(None);
    };
    if value.is_null() {
        return Ok(None);
    }
    let text = value
        .as_str()
        .ok_or_else(|| AppError::InvalidArgument(format!("{context}.{key} 必须是字符串")))?
        .trim();
    if text.is_empty() {
        return Ok(None);
    }
    Ok(Some(text))
}

pub(crate) fn optional_value_field(json: &Value, key: &str) -> Option<Value> {
    json.get(key).filter(|value| !value.is_null()).cloned()
}

pub(super) fn required_field<'a>(
    json: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Value, AppError> {
    json.get(key)
        .ok_or_else(|| AppError::InvalidArgument(format!("{context} 缺少必需字段 {key}")))
}
