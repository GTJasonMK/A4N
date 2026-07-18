use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use a4n_orchestrator::generation::generation_checkpoint_fingerprint_sha256;
use serde_json::{Value, json};

pub(super) fn character_conflict_workflow_fingerprint(
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": "character_conflict_check",
        "schema_version": "character_conflict_workflow_v1",
        "project_id": project_id,
        "character_name": name,
        "chapter_scope": {
            "start_chapter": start_chapter,
            "end_chapter": end_chapter,
        },
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}

pub(super) fn character_conflict_business_key(fingerprint: &Value) -> Result<String, AppError> {
    Ok(format!(
        "character_conflict_check:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

pub(super) fn character_conflict_call_fingerprint_json(
    call_key: &str,
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "character_conflict_call_v1",
        "workflow": "character_conflict_check",
        "call_key": call_key,
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}
