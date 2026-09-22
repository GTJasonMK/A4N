use a4n_error::AppError;
use serde_json::Value;

pub(in crate::manga::pipeline::prompt::validation) fn reject_manga_unknown_fields(
    value: &Value,
    allowed: &[&str],
    path: &str,
) -> Result<(), AppError> {
    let object = value
        .as_object()
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是 JSON 对象")))?;
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "{path} 包含未知字段 {key}；允许字段：{}",
                allowed.join(", ")
            )));
        }
    }
    Ok(())
}

pub(in crate::manga::pipeline::prompt) fn required_manga_string(
    value: &Value,
    key: &str,
    path: &str,
) -> Result<String, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path}.{key} 必须是非空字符串")))
}

pub(in crate::manga::pipeline::prompt::validation) fn required_manga_enum(
    value: &Value,
    key: &str,
    path: &str,
    allowed: &[&str],
) -> Result<(), AppError> {
    let actual = required_manga_string(value, key, path)?;
    if allowed.contains(&actual.as_str()) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{path}.{key} 必须是 {} 之一",
            allowed.join("、")
        )))
    }
}

pub(in crate::manga::pipeline::prompt::validation) fn required_manga_bool(
    value: Option<&Value>,
    path: &str,
) -> Result<(), AppError> {
    if value.and_then(Value::as_bool).is_some() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{path} 必须是布尔值")))
    }
}

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_nullable_string(
    value: Option<&Value>,
    path: &str,
) -> Result<(), AppError> {
    match value {
        Some(Value::Null) => Ok(()),
        Some(Value::String(text)) if !text.trim().is_empty() => Ok(()),
        Some(Value::String(_)) => Err(AppError::InvalidArgument(format!(
            "{path} 必须是非空字符串或 null"
        ))),
        Some(_) => Err(AppError::InvalidArgument(format!(
            "{path} 必须是字符串或 null"
        ))),
        None => Err(AppError::InvalidArgument(format!("{path} 缺失"))),
    }
}

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_string_array(
    value: Option<&Value>,
    path: &str,
) -> Result<(), AppError> {
    let values = value
        .and_then(Value::as_array)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是数组")))?;
    for (index, value) in values.iter().enumerate() {
        let text = value.as_str().map(str::trim).ok_or_else(|| {
            AppError::InvalidArgument(format!("{path}[{index}] 必须是非空字符串"))
        })?;
        if text.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "{path}[{index}] 必须是非空字符串"
            )));
        }
    }
    Ok(())
}

pub(in crate::manga::pipeline::prompt::validation) fn validate_manga_positive_i64(
    value: Option<&Value>,
    path: &str,
) -> Result<i64, AppError> {
    let number = value
        .and_then(Value::as_i64)
        .ok_or_else(|| AppError::InvalidArgument(format!("{path} 必须是正整数")))?;
    if number <= 0 {
        return Err(AppError::InvalidArgument(format!("{path} 必须是正整数")));
    }
    Ok(number)
}
