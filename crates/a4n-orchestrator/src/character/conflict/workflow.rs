use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;

use crate::{
    character::{
        conflict_context_rules::CharacterConflictChapterScope,
        conflict_rules::{
            character_conflict_business_key, character_conflict_workflow_fingerprint,
        },
    },
    workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING},
};

use super::CHARACTER_CONFLICT_KIND;

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
