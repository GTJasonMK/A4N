use a4n_core::error::AppError;
use serde_json::Value;

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

use super::super::super::chapter_contract::parse_chapter_outline_batch;
use super::preparation::PreparedBeatMapping;

pub(super) fn restored_beat_mapping_checkpoint(
    parsed_output: &Value,
    response_content: &str,
    prepared: &PreparedBeatMapping,
) -> Result<Vec<Value>, AppError> {
    let parsed = parse_chapter_outline_batch(
        response_content,
        prepared.part.start_chapter,
        prepared.count,
    )?;
    ensure_checkpoint_parsed_output_matches(
        parsed_output,
        &Value::Array(parsed.clone()),
        &format!("第 {} 部节拍映射", prepared.part.part_number),
    )?;
    Ok(parsed)
}
