use a4n_core::error::AppError;
use serde::Deserialize;
use serde_json::Value;

use super::invalid;

pub(in crate::galgame::pipeline_json) fn require_object_value(
    value: Value,
    context: &str,
) -> Result<Value, AppError> {
    if value.as_object().is_some() {
        Ok(value)
    } else {
        Err(invalid(format!("{context} 不是 JSON 对象")))
    }
}

pub(in crate::galgame::pipeline_json) fn from_value<T>(
    value: Value,
    context: &str,
) -> Result<T, AppError>
where
    T: for<'de> Deserialize<'de>,
{
    value
        .as_object()
        .ok_or_else(|| invalid(format!("{context} 不是 JSON 对象")))?;
    serde_json::from_value(value)
        .map_err(|err| invalid(format!("{context} JSON 契约不匹配: {err}")))
}
