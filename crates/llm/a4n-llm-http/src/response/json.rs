use super::super::errors::{internal_error, response_preview};
use a4n_error::AppError;
use serde_json::Value;

pub(super) fn json_response_preview(value: &Value, max_chars: usize) -> String {
    response_preview(&value.to_string(), max_chars)
}

#[cfg(test)]
pub(crate) fn parse_response_json(body: &str, label: &str) -> Result<Value, AppError> {
    serde_json::from_str::<Value>(body).map_err(|error| {
        internal_error(format!(
            "解析{label}响应 JSON 失败: {error}; 响应片段: {}",
            response_preview(body, 500)
        ))
    })
}

pub(crate) fn required_object<'a>(
    value: &'a Value,
    field: &str,
) -> Result<&'a serde_json::Map<String, Value>, AppError> {
    value
        .as_object()
        .ok_or_else(|| internal_error(format!("LLM 响应缺少对象字段 {field}")))
}

pub(crate) fn required_usize(value: &Value, field: &str) -> Result<usize, AppError> {
    let number = value
        .as_u64()
        .ok_or_else(|| internal_error(format!("LLM 响应缺少非负整数字段 {field}")))?;
    usize::try_from(number)
        .map_err(|_| internal_error(format!("LLM 响应字段 {field} 超出 usize 范围")))
}

pub(crate) fn optional_usize(
    value: Option<&Value>,
    field: &str,
) -> Result<Option<usize>, AppError> {
    value.map(|value| required_usize(value, field)).transpose()
}

pub(crate) fn optional_nested_usize(
    value: &Value,
    path: &[&str],
    field: &str,
) -> Result<Option<usize>, AppError> {
    let mut current = value;
    for key in path {
        let Some(next) = current.get(*key) else {
            return Ok(None);
        };
        current = next;
    }
    required_usize(current, field).map(Some)
}

pub(super) fn non_empty_string(value: &Value) -> Option<&str> {
    value
        .as_str()
        .map(str::trim)
        .filter(|text| !text.is_empty())
}
