use a4n_orchestrator::shooting_script::{
    ChapterShootingScriptSourceInput, RegenerateChapterShootingScriptInput,
};
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;

use super::contracts::{ChapterShootingScriptView, GenerateChapterShootingScriptInput};
use crate::{AppState, actions::access};

pub async fn generate_chapter_shooting_script(
    state: &AppState,
    user_id: i64,
    input: GenerateChapterShootingScriptInput,
    progress: &(dyn Fn(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send + Sync),
    cancel_token: Option<&tokio_util::sync::CancellationToken>,
) -> Result<ChapterShootingScriptView, String> {
    access::verify_project_or_derived_family_editor(state, user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let target_revision = state
        .orchestrator
        .next_chapter_shooting_script_revision(ChapterShootingScriptSourceInput {
            project_id: input.project_id,
            source_chapter_id: input.source_chapter_id,
        })
        .await
        .map_err(|error| error.to_string())?;
    let workflow = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!(
                "chapter_shooting_script:{}:{}:{}",
                input.project_id, input.source_chapter_id, target_revision
            ),
            workflow_kind: "chapter_shooting_script".to_string(),
            project_id: Some(input.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("chapter_shooting_script".to_string()),
            scope_id: Some(input.source_chapter_id),
            chapter_id: Some(input.source_chapter_id),
            mode: None,
            input_fingerprint_json: Some(json!({
                "project_id": input.project_id,
                "source_chapter_id": input.source_chapter_id,
            })),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("screenplay_layout".to_string()),
            active_task_id: None,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|error| error.to_string())?;
    let result = state
        .orchestrator
        .regenerate_chapter_shooting_script_with_progress(
            RegenerateChapterShootingScriptInput {
                project_id: input.project_id,
                source_chapter_id: input.source_chapter_id,
                workflow_run_id: workflow.id,
            },
            progress,
            cancel_token,
        )
        .await;
    let resolved = match result {
        Ok(value) => {
            state
                .orchestrator
                .mark_workflow_status(workflow.id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await
                .map_err(|error| error.to_string())?;
            value
        }
        Err(error) => {
            let status = if matches!(error, a4n_core::error::AppError::Cancelled) {
                WORKFLOW_STATUS_CANCELLED
            } else {
                WORKFLOW_STATUS_FAILED
            };
            state
                .orchestrator
                .mark_workflow_status(workflow.id, status, true, Some(&error.to_string()))
                .await
                .map_err(|status_error| format!("{error}; 更新工作流状态失败: {status_error}"))?;
            return Err(error.to_string());
        }
    };
    Ok(ChapterShootingScriptView {
        record: resolved.shooting_script.record,
        script: resolved.shooting_script.script,
    })
}
