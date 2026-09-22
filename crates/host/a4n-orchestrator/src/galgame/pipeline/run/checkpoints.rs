use a4n_error::AppError;
use serde_json::Value;

use crate::structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response};

pub(super) fn restored_galgame_llm_stage_output<T, F>(
    checkpoint_parsed_output: &Value,
    response_content: &str,
    checkpoint_key: &str,
    prompt_name: &str,
    validate: F,
) -> Result<(Value, T), AppError>
where
    T: serde::Serialize,
    F: Fn(Value) -> Result<T, AppError>,
{
    let reparsed = validate(parse_strict_json_response(response_content, prompt_name)?)?;
    let parsed_json = serde_json::to_value(&reparsed)?;
    ensure_checkpoint_parsed_output_matches(
        checkpoint_parsed_output,
        &parsed_json,
        &format!("Galgame 管线 {checkpoint_key}"),
    )?;
    Ok((parsed_json, reparsed))
}
