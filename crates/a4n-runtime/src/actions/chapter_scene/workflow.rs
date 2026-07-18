use super::contracts::GenerateChapterSceneInput;
use super::output::normalize_scene_generation_output;
use crate::AppState;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_orchestrator::generation::generation_checkpoint_fingerprint_sha256;
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};
use serde_json::{Value, json};

pub(super) fn chapter_scene_workflow_fingerprint(
    input: &GenerateChapterSceneInput,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "chapter_scene_generation",
        "schema_version": "chapter_scene_generation_workflow_v1",
        "chapter_id": input.chapter_id,
        "scene_number": input.scene_number,
        "scene_prompt": input.scene_prompt.trim(),
        "request": context_aware_chat_request_trace_json(request)?}))
}

pub(super) fn chapter_scene_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "chapter_scene_generation:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

pub(super) fn chapter_scene_call_fingerprint_json(
    call_key: &str,
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "chapter_scene_generation_call_v1",
        "workflow": "chapter_scene_generation",
        "call_key": call_key,
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)?}))
}

pub(super) async fn finalize_chapter_scene_workflow<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<T, AppError>,
) -> Result<(), AppError> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(err) => (WORKFLOW_STATUS_FAILED, true, Some(err.to_string())),
    };
    state
        .orchestrator
        .mark_workflow_status(
            workflow_run_id,
            status,
            can_resume,
            error_message.as_deref(),
        )
        .await?;
    Ok(())
}

pub(super) fn restored_chapter_scene_checkpoint(
    checkpoint_json: Value,
    call_key: &str,
) -> Result<String, AppError> {
    let parsed_output = checkpoint_json
        .get("parsed_output")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:parsed_output".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        })?;
    let response_content = checkpoint_json
        .get("response_content")
        .and_then(Value::as_str)
        .ok_or_else(|| AppError::InvalidState {
            current: "missing_or_invalid_checkpoint_field:response_content".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        })?;
    let reparsed = normalize_scene_generation_output(response_content)?;
    if reparsed != parsed_output {
        return Err(AppError::InvalidState {
            current: "checkpoint_parsed_output_mismatch".to_string(),
            operation: format!("恢复章节场景生成 LLM 调用断点 {call_key}"),
        });
    }
    Ok(reparsed)
}
