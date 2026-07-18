use serde_json::Value;

use super::super::super::ApiError;

pub(in crate::routes::commands) fn json_i64_vec(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i64>, ApiError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 缺失或不是数组")))?;
    array
        .iter()
        .map(|item| {
            item.as_i64()
                .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 必须是整数数组")))
        })
        .collect()
}

pub(in crate::routes::commands) fn optional_json_i64_vec(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i64>, ApiError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(_)) => json_i64_vec(value, key, label),
        Some(_) => Err(ApiError::bad_request(format!(
            "{label}.{key} 必须是整数数组或 null"
        ))),
    }
}

pub(in crate::routes::commands) fn json_i32_vec(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i32>, ApiError> {
    let array = value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 缺失或不是数组")))?;
    array
        .iter()
        .map(|item| {
            let raw = item
                .as_i64()
                .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 必须是整数数组")))?;
            i32::try_from(raw).map_err(|_| {
                ApiError::bad_request(format!("{label}.{key} 包含超出 i32 范围的整数"))
            })
        })
        .collect()
}

pub(in crate::routes::commands) fn optional_json_i32_vec(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i32>, ApiError> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(_)) => json_i32_vec(value, key, label),
        Some(_) => Err(ApiError::bad_request(format!(
            "{label}.{key} 必须是整数数组或 null"
        ))),
    }
}
