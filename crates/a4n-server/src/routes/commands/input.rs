use a4n_runtime::host_api::dependency_conflict::DownstreamConflictStrategy;
use serde::Deserialize;
use serde_json::Value;

use super::ApiError;

pub(in crate::routes::commands) fn verify_user_arg(
    args: &Value,
    authenticated_user_id: i64,
) -> Result<(), ApiError> {
    let Some(value) = args.get("userId") else {
        return Ok(());
    };
    let Some(user_id) = value.as_i64() else {
        return Err(ApiError::bad_request("userId 必须是整数"));
    };
    if user_id != authenticated_user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    Ok(())
}

pub(in crate::routes::commands) fn required_arg<'a>(
    args: &'a Value,
    key: &str,
) -> Result<&'a Value, ApiError> {
    args.get(key)
        .ok_or_else(|| ApiError::bad_request(format!("缺少参数: {key}")))
}

pub(in crate::routes::commands) fn i64_arg(args: &Value, key: &str) -> Result<i64, ApiError> {
    required_arg(args, key)?
        .as_i64()
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是正整数")))
}

pub(in crate::routes::commands) fn bool_arg(args: &Value, key: &str) -> Result<bool, ApiError> {
    required_arg(args, key)?
        .as_bool()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是布尔值")))
}

pub(in crate::routes::commands) fn nullable_i64_arg(
    args: &Value,
    key: &str,
) -> Result<Option<i64>, ApiError> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => value
            .as_i64()
            .filter(|value| *value > 0)
            .map(Some)
            .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是正整数或 null"))),
    }
}

pub(in crate::routes::commands) fn u64_arg(args: &Value, key: &str) -> Result<u64, ApiError> {
    required_arg(args, key)?
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是正整数")))
}

pub(in crate::routes::commands) fn non_negative_u64_arg(
    args: &Value,
    key: &str,
) -> Result<u64, ApiError> {
    required_arg(args, key)?
        .as_u64()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是非负整数")))
}

pub(in crate::routes::commands) fn f32_arg(args: &Value, key: &str) -> Result<f32, ApiError> {
    let value = required_arg(args, key)?
        .as_f64()
        .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是数字")))?;
    if !value.is_finite() || value < f32::MIN as f64 || value > f32::MAX as f64 {
        return Err(ApiError::bad_request(format!("{key} 必须是有限 f32 数字")));
    }
    Ok(value as f32)
}

pub(in crate::routes::commands) fn string_arg(args: &Value, key: &str) -> Result<String, ApiError> {
    let value = required_arg(args, key)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| ApiError::bad_request(format!("{key} 不能为空")))?;
    Ok(value.to_string())
}

pub(in crate::routes::commands) fn optional_string_field(
    input: &Value,
    key: &str,
) -> Result<Option<String>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => Err(ApiError::bad_request(format!("{key} 必须是字符串或 null"))),
    }
}

pub(in crate::routes::commands) fn downstream_strategy_arg(
    args: &Value,
) -> Result<DownstreamConflictStrategy, ApiError> {
    DownstreamConflictStrategy::from_optional_str(
        optional_string_field(args, "downstreamStrategy")?.as_deref(),
    )
    .map_err(ApiError::bad_request)
}

pub(in crate::routes::commands) fn optional_bool_field(
    input: &Value,
    key: &str,
) -> Result<bool, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Bool(value)) => Ok(*value),
        Some(_) => Err(ApiError::bad_request(format!("{key} 必须是布尔值或 null"))),
    }
}

pub(in crate::routes::commands) fn optional_i32_field(
    input: &Value,
    key: &str,
) -> Result<Option<i32>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(value) => {
            let raw = value
                .as_i64()
                .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是整数或 null")))?;
            i32::try_from(raw)
                .map(Some)
                .map_err(|_| ApiError::bad_request(format!("{key} 超出 i32 范围")))
        }
    }
}

pub(in crate::routes::commands) fn optional_string_vec_field(
    input: &Value,
    key: &str,
) -> Result<Option<Vec<String>>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_str()
                    .map(str::to_string)
                    .ok_or_else(|| ApiError::bad_request(format!("{key} 必须是字符串数组或 null")))
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Some),
        Some(_) => Err(ApiError::bad_request(format!(
            "{key} 必须是字符串数组或 null"
        ))),
    }
}

pub(in crate::routes::commands) fn optional_json_i64_vec(
    input: &Value,
    key: &str,
    label: &str,
) -> Result<Vec<i64>, ApiError> {
    match input.get(key) {
        None | Some(Value::Null) => Ok(Vec::new()),
        Some(Value::Array(items)) => items
            .iter()
            .map(|item| {
                item.as_i64()
                    .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 必须是整数数组")))
            })
            .collect(),
        Some(_) => Err(ApiError::bad_request(format!(
            "{label}.{key} 必须是整数数组或 null"
        ))),
    }
}

pub(in crate::routes::commands) fn to_value<T: serde::Serialize>(
    value: T,
) -> Result<Value, ApiError> {
    serde_json::to_value(value)
        .map_err(|e| ApiError::bad_request(format!("序列化 Web command 响应失败: {e}")))
}

pub(in crate::routes::commands) fn command_input<T>(
    args: &Value,
    command: &str,
) -> Result<T, ApiError>
where
    T: for<'de> Deserialize<'de>,
{
    serde_json::from_value(required_arg(args, "input")?.clone())
        .map_err(|e| ApiError::bad_request(format!("{command}.input 无效: {e}")))
}

pub(in crate::routes::commands) fn required_json_i32(
    value: &Value,
    key: &str,
    label: &str,
) -> Result<i32, ApiError> {
    let raw = value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| ApiError::bad_request(format!("{label}.{key} 缺失或不是整数")))?;
    i32::try_from(raw).map_err(|_| ApiError::bad_request(format!("{label}.{key} 超出 i32 范围")))
}
