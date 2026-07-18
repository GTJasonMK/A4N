use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use super::super::super::part_contract::validate_materialized_part_outline;
use super::super::contracts::GeneratedPartOutline;

pub(in crate::outline::parts) fn restored_part_outline_checkpoint(
    checkpoint_json: Value,
    request: ChatRequest,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<GeneratedPartOutline, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取第 {part_num} 部分部大纲调用级断点"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取第 {part_num} 部分部大纲调用级断点"),
        })?
        .to_string();
    let finish_reason = checkpoint_json
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::to_string);
    let parsed = validate_materialized_part_outline(&parsed_output, part_num, start_ch, end_ch)?;
    Ok(GeneratedPartOutline {
        data: parsed,
        request,
        response_content,
        finish_reason,
    })
}
