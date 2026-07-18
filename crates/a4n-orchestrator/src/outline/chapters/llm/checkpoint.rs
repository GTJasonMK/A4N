use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use crate::outline::chapter_contract::validate_materialized_chapter_outline_batch;

use super::super::contracts::GeneratedChapterOutline;

pub(super) fn chapter_outline_call_fingerprint_json(
    call_key: &str,
    chapter_number: i32,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": 1,
        "workflow": "chapter_outline_generation",
        "call_key": call_key,
        "prompt_name": "outline",
        "messages": request.messages,
        "context_manifest": request.context_manifest_json,
        "max_tokens": request.max_tokens,
        "max_tokens_profile": request.max_tokens_profile,
        "response_format": request.response_format,
        "target": {
            "chapter_number": chapter_number,
            "count": 1,
        },
    }))
}

pub(super) fn restored_chapter_outline_checkpoint(
    checkpoint_json: Value,
    request: ChatRequest,
    chapter_number: i32,
) -> Result<GeneratedChapterOutline, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_parsed_output".to_string(),
            operation: format!("读取第 {chapter_number} 章章纲调用级断点"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_checkpoint_response_content".to_string(),
            operation: format!("读取第 {chapter_number} 章章纲调用级断点"),
        })?
        .to_string();
    let finish_reason = checkpoint_json
        .get("finish_reason")
        .and_then(Value::as_str)
        .map(str::to_string);
    let mut parsed = validate_materialized_chapter_outline_batch(
        &Value::Array(vec![parsed_output.clone()]),
        chapter_number,
        1,
    )?;
    let parsed = parsed.pop().ok_or_else(|| {
        AppError::InvalidArgument(format!("第 {chapter_number} 章章纲断点结果为空"))
    })?;
    Ok(GeneratedChapterOutline {
        chapter_json: parsed,
        request,
        response_content,
        finish_reason,
    })
}
