use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use crate::outline::chapter_contract::parse_chapter_outline_batch;
use crate::structured_llm::ensure_checkpoint_parsed_output_matches;

use super::super::contracts::GeneratedChapterOutline;

pub(super) fn chapter_outline_call_fingerprint_json(
    call_key: &str,
    chapter_number: i32,
    request: &ChatRequest,
    source_snapshot: &Value,
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
        "source_snapshot_sha256": a4n_core::source_spans::source_text_hash(
            &serde_json::to_string(source_snapshot)?
        ),
        "target": {
            "chapter_number": chapter_number,
            "count": 1,
        },
    }))
}

pub(super) fn restored_chapter_outline_checkpoint(
    parsed_output: Value,
    response_content: String,
    finish_reason: Option<String>,
    request: ChatRequest,
    chapter_number: i32,
) -> Result<GeneratedChapterOutline, AppError> {
    let mut parsed = parse_chapter_outline_batch(&response_content, chapter_number, 1)?;
    let parsed = parsed.pop().ok_or_else(|| {
        AppError::InvalidArgument(format!("第 {chapter_number} 章章纲断点结果为空"))
    })?;
    ensure_checkpoint_parsed_output_matches(
        &parsed_output,
        &parsed,
        &format!("第 {chapter_number} 章章纲"),
    )?;
    Ok(GeneratedChapterOutline {
        chapter_json: parsed,
        request,
        response_content,
        finish_reason,
    })
}
