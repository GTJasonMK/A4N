use a4n_core::error::AppError;
use serde_json::Value;

use super::super::super::chapter_contract::validate_materialized_chapter_outline_batch;
use super::preparation::PreparedBeatMapping;

pub(super) fn restored_beat_mapping_checkpoint(
    checkpoint_json: Value,
    prepared: &PreparedBeatMapping,
) -> Result<Vec<Value>, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取第 {} 部节拍映射调用级断点", prepared.part.part_number),
        })?;
    checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取第 {} 部节拍映射调用级断点", prepared.part.part_number),
        })?;
    let parsed = validate_materialized_chapter_outline_batch(
        &parsed_output,
        prepared.part.start_chapter,
        prepared.count,
    )?;
    Ok(parsed)
}
