use a4n_error::AppError;
use serde::Deserialize;
use serde_json::Value;

use super::invalid;

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
