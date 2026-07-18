use a4n_core::error::AppError;
use serde_json::Value;

pub(super) fn restored_galgame_llm_stage_output<F>(
    checkpoint_json: &Value,
    checkpoint_key: &str,
    _prompt_name: &str,
    validate: F,
) -> Result<Value, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    let checkpoint_parsed_output =
        required_checkpoint_value(checkpoint_json, checkpoint_key, "parsed_output")?;
    required_checkpoint_string(checkpoint_json, checkpoint_key, "response_content")?;
    validate(checkpoint_parsed_output)
}

fn required_checkpoint_value(
    checkpoint_json: &Value,
    checkpoint_key: &str,
    field: &str,
) -> Result<Value, AppError> {
    checkpoint_json
        .get(field)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{checkpoint_key}.{field}_missing"),
            operation: format!("读取 Galgame 管线 checkpoint {checkpoint_key}"),
        })
}

fn required_checkpoint_string(
    checkpoint_json: &Value,
    checkpoint_key: &str,
    field: &str,
) -> Result<String, AppError> {
    checkpoint_json
        .get(field)
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("{checkpoint_key}.{field}_missing_or_invalid"),
            operation: format!("读取 Galgame 管线 checkpoint {checkpoint_key}"),
        })
}
