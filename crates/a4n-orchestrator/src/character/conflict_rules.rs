//! 角色冲突检查工作流指纹纯规则。

use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_llm::trace::context_aware_chat_request_trace_json;
use serde_json::{Value, json};

use crate::{
    character::conflict::CHARACTER_CONFLICT_KIND,
    generation::generation_checkpoint_fingerprint_sha256,
};

pub(in crate::character) fn character_conflict_workflow_fingerprint(
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "workflow": CHARACTER_CONFLICT_KIND,
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

pub(in crate::character) fn character_conflict_business_key(
    fingerprint: &Value,
) -> Result<String, AppError> {
    Ok(format!(
        "{CHARACTER_CONFLICT_KIND}:{}",
        generation_checkpoint_fingerprint_sha256(fingerprint)?
    ))
}

pub(in crate::character) fn character_conflict_call_fingerprint_json(
    call_key: &str,
    workflow_fingerprint: &Value,
    request: &ChatRequest,
) -> Result<Value, AppError> {
    Ok(json!({
        "schema_version": "character_conflict_call_v1",
        "workflow": CHARACTER_CONFLICT_KIND,
        "call_key": call_key,
        "workflow_fingerprint": workflow_fingerprint,
        "request": context_aware_chat_request_trace_json(request)?,
    }))
}
