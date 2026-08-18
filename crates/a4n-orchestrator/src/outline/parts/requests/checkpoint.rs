use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

use super::super::super::part_contract::parse_part_outline_single_response;
use super::super::contracts::GeneratedPartOutline;

pub(in crate::outline::parts) fn restored_part_outline_checkpoint(
    parsed_output: Value,
    response_content: String,
    finish_reason: Option<String>,
    request: ChatRequest,
    part_num: i32,
    start_ch: i32,
    end_ch: i32,
) -> Result<GeneratedPartOutline, AppError> {
    let parsed =
        parse_part_outline_single_response(&response_content, part_num, start_ch, end_ch)?;
    ensure_checkpoint_parsed_output_matches(
        &parsed_output,
        &parsed,
        &format!("第 {part_num} 部分部大纲"),
    )?;
    Ok(GeneratedPartOutline {
        data: parsed,
        request,
        response_content,
        finish_reason,
    })
}
