use serde::Deserialize;
use serde_json::Value;

use super::ApiError;

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

pub(in crate::routes::commands) fn strict_command_input<T>(
    args: &Value,
    command: &str,
) -> Result<T, ApiError>
where
    T: for<'de> Deserialize<'de>,
{
    reject_unknown_command_args(args, command, &["input"])?;
    command_input(args, command)
}

pub(in crate::routes::commands) fn strict_runtime_command_input<T>(
    args: &Value,
    command: &str,
) -> Result<T, ApiError>
where
    T: for<'de> Deserialize<'de>,
{
    reject_unknown_command_args(args, command, &["input", "runtime_config_snapshot"])?;
    command_input(args, command)
}

pub(in crate::routes::commands) fn strict_optional_task_runtime_command_input<T>(
    args: &Value,
    command: &str,
) -> Result<T, ApiError>
where
    T: for<'de> Deserialize<'de>,
{
    reject_unknown_command_args(
        args,
        command,
        &["input", "runtime_config_snapshot", "taskId"],
    )?;
    command_input(args, command)
}

pub(in crate::routes::commands) fn reject_unknown_command_args(
    args: &Value,
    command: &str,
    allowed_fields: &[&str],
) -> Result<(), ApiError> {
    let fields = args
        .as_object()
        .ok_or_else(|| ApiError::bad_request(format!("{command} 参数必须是对象")))?;
    if let Some(field) = fields
        .keys()
        .find(|field| !allowed_fields.contains(&field.as_str()))
    {
        return Err(ApiError::bad_request(format!(
            "{command} 不接受参数: {field}"
        )));
    }
    Ok(())
}

pub(in crate::routes::commands) fn empty_command_args(
    args: &Value,
    command: &str,
) -> Result<(), ApiError> {
    match args.as_object() {
        Some(args) if args.is_empty() => Ok(()),
        _ => Err(ApiError::bad_request(format!("{command} 不接受参数"))),
    }
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

#[cfg(test)]
mod tests {
    use super::{
        reject_unknown_command_args, strict_command_input,
        strict_optional_task_runtime_command_input, strict_runtime_command_input,
    };
    use serde::Deserialize;
    use serde_json::json;

    #[derive(Deserialize)]
    struct TestInput {
        value: u64,
    }

    #[test]
    fn task_workflow_strict_inputs_reject_identity_args_and_allow_runtime_snapshot() {
        let input =
            strict_command_input::<TestInput>(&json!({ "input": { "value": 7 } }), "test_command")
                .unwrap();
        assert_eq!(input.value, 7);

        assert!(
            strict_command_input::<TestInput>(
                &json!({ "userId": 9, "input": { "value": 7 } }),
                "test_command",
            )
            .is_err()
        );
        assert!(
            strict_command_input::<TestInput>(
                &json!({ "runtime_config_snapshot": null, "input": { "value": 7 } }),
                "test_command",
            )
            .is_err()
        );
        assert!(
            strict_runtime_command_input::<TestInput>(
                &json!({ "runtime_config_snapshot": null, "input": { "value": 7 } }),
                "test_command",
            )
            .is_ok()
        );
        assert!(
            strict_optional_task_runtime_command_input::<TestInput>(
                &json!({
                    "input": { "value": 7 },
                    "taskId": "scene-sync",
                    "runtime_config_snapshot": null
                }),
                "test_command",
            )
            .is_ok()
        );
        assert!(
            strict_optional_task_runtime_command_input::<TestInput>(
                &json!({ "input": { "value": 7 }, "projectId": 9 }),
                "test_command",
            )
            .is_err()
        );
        assert!(reject_unknown_command_args(&json!({ "input": {} }), "empty", &[]).is_err());
    }
}
