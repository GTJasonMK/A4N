use super::CHARACTER_CONFLICT_KIND;
use crate::{
    character::conflict::context_rules::CharacterConflictChapterScope,
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING},
};
use a4n_db::generation::generation_checkpoint_fingerprint_sha256;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_llm_trace::context_aware_chat_request_trace_json;
use serde_json::Value;
use serde_json::json;

pub(super) fn character_conflict_workflow_run_input(
    project_id: i64,
    profile_id: i64,
    name: &str,
    chapter_scope: CharacterConflictChapterScope,
    call_key: &str,
    request: &ChatRequest,
    active_task_id: Option<&str>,
) -> Result<(UpsertWorkflowRunInput, Value), AppError> {
    let fingerprint = character_conflict_workflow_fingerprint(
        project_id,
        name,
        chapter_scope.start_chapter,
        chapter_scope.end_chapter,
        request,
    )?;
    let business_key = character_conflict_business_key(&fingerprint)?;
    Ok((
        UpsertWorkflowRunInput {
            business_key,
            workflow_kind: CHARACTER_CONFLICT_KIND.to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: Some(profile_id),
            chapter_id: None,
            mode: Some("check".to_string()),
            input_fingerprint_json: Some(fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(call_key.to_string()),
            active_task_id: active_task_id.map(str::to_string),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        },
        fingerprint,
    ))
}

// 角色冲突检查工作流指纹纯规则。

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
