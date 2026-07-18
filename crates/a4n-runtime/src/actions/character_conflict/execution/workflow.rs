use crate::{AppState, actions::access};
use a4n_core::error::AppError;
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED,
    WORKFLOW_STATUS_RUNNING,
};
use serde_json::Value;

use super::{
    call::run_character_conflict_llm_call,
    rules::{character_conflict_business_key, character_conflict_workflow_fingerprint},
};
use crate::actions::character_conflict::{
    CharacterConflictProgressReporter, CharacterConflictTaskScope,
    request::build_character_conflict_request,
};

pub async fn character_conflict_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<CharacterConflictTaskScope, AppError> {
    access::verify_project_editor(state, user_id, project_id).await?;
    Ok(CharacterConflictTaskScope {
        project_id,
        total: 4,
    })
}

pub async fn check_character_conflict(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
    progress: CharacterConflictProgressReporter<'_>,
) -> Result<Vec<Value>, AppError> {
    progress(0, 4, "正在读取角色档案和章节内容")?;
    let context = build_character_conflict_request(
        state,
        user_id,
        project_id,
        name,
        start_chapter,
        end_chapter,
    )
    .await?;
    progress(1, 4, "正在构造一致性检查提示词")?;
    progress(2, 4, "正在调用模型检查角色一致性")?;
    let workflow_fingerprint = character_conflict_workflow_fingerprint(
        project_id,
        name,
        start_chapter,
        end_chapter,
        &context.request,
    )?;
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: character_conflict_business_key(&workflow_fingerprint)?,
            workflow_kind: "character_conflict_check".to_string(),
            project_id: Some(project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("protagonist_profile".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some("check".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(context.call_key.clone()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await?;
    let result = run_character_conflict_llm_call(
        state,
        workflow_run.id,
        workflow_fingerprint,
        context,
        progress,
    )
    .await;
    finalize_character_conflict_workflow(state, workflow_run.id, &result).await?;
    result
}

async fn finalize_character_conflict_workflow<T>(
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
