use a4n_error::AppError;
use serde_json::Value;

use super::specs::INFORMATION_SOURCES;

pub(super) fn required_string<'a>(input: &'a Value, field: &str) -> Result<&'a str, AppError> {
    input
        .get(field)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument(format!("缺少必填字符串参数: {field}")))
}

pub(super) fn required_information_source<'a>(
    input: &'a Value,
    field: &str,
) -> Result<&'a str, AppError> {
    let source = required_string(input, field)?;
    if INFORMATION_SOURCES.contains(&source) {
        Ok(source)
    } else {
        Err(AppError::InvalidArgument(format!("未知信息源: {source}")))
    }
}

pub(super) fn optional_information_source<'a>(
    input: &'a Value,
    field: &str,
) -> Result<Option<&'a str>, AppError> {
    match input.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            let source = value.trim();
            if INFORMATION_SOURCES.contains(&source) {
                Ok(Some(source))
            } else {
                Err(AppError::InvalidArgument(format!("未知信息源: {source}")))
            }
        }
        Some(_) => Err(AppError::InvalidArgument(format!("{field} 必须是字符串"))),
    }
}

pub(super) fn optional_limit(input: &Value) -> Result<usize, AppError> {
    match input.get("limit") {
        None | Some(Value::Null) => Ok(8),
        Some(Value::Number(value)) => {
            let limit = value
                .as_u64()
                .ok_or_else(|| AppError::InvalidArgument("limit 必须是正整数".into()))?;
            if (1..=20).contains(&limit) {
                Ok(limit as usize)
            } else {
                Err(AppError::InvalidArgument("limit 必须在 1..=20 之间".into()))
            }
        }
        Some(_) => Err(AppError::InvalidArgument("limit 必须是整数".into())),
    }
}
