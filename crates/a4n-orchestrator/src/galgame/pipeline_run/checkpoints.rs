use a4n_error::AppError;
use serde_json::Value;

use crate::structured_llm::{ensure_checkpoint_parsed_output_matches, parse_strict_json_response};

pub(super) fn restored_galgame_llm_stage_output<F>(
    checkpoint_parsed_output: &Value,
    response_content: &str,
    checkpoint_key: &str,
    prompt_name: &str,
    validate: F,
) -> Result<Value, AppError>
where
    F: Fn(Value) -> Result<Value, AppError>,
{
    let reparsed = validate(parse_strict_json_response(response_content, prompt_name)?)?;
    ensure_checkpoint_parsed_output_matches(
        checkpoint_parsed_output,
        &reparsed,
        &format!("Galgame 管线 {checkpoint_key}"),
    )?;
    Ok(reparsed)
}
