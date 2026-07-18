use super::super::contracts::{
    GALGAME_PIPELINE_MODE_FULL, GALGAME_PIPELINE_WORKFLOW_KIND, StartGalgamePipelineInput,
};
use crate::AppState;
use crate::actions::galgame_records::project_record_by_id;
use crate::actions::{access, workflow_status_write_error};
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_orchestrator::galgame::{
    GalgamePipelineRunOptions, GalgamePipelineStartStage, GalgameProgressReporter,
    GalgameScriptDetail, parse_galgame_start_stage,
};
use a4n_orchestrator::workflow::{
    UpsertWorkflowRunInput, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;

pub async fn start_galgame_pipeline(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
) -> Result<GalgameScriptDetail, String> {
    start_galgame_pipeline_with_progress(state, user_id, input, None).await
}

pub async fn start_galgame_pipeline_with_progress(
    state: &AppState,
    user_id: i64,
    input: StartGalgamePipelineInput,
    progress_reporter: Option<GalgameProgressReporter>,
) -> Result<GalgameScriptDetail, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    let start_from_stage = parse_galgame_start_stage(input.start_from_stage.as_deref())
        .map_err(|err| err.to_string())?;
    let project = project_record_by_id(state, input.project_id).await?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| format!("Galgame 项目缺少源文本快照: {}", project.id))?;
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| format!("Galgame 项目缺少根文本项目: {}", project.id))?;
    let fingerprint = json!({
        "project_id": project.id,
        "source_snapshot_id": source_snapshot_id,
        "root_project_id": root_project_id,
        "mode": GALGAME_PIPELINE_MODE_FULL,
    });
    let initial_stage =
        start_from_stage.unwrap_or(GalgamePipelineStartStage::ShootingScriptProjection);
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: format!("galgame_pipeline:{}", project.id),
            workflow_kind: GALGAME_PIPELINE_WORKFLOW_KIND.to_string(),
            project_id: Some(project.id),
            root_project_id: Some(root_project_id),
            source_snapshot_id: Some(source_snapshot_id),
            scope_table: Some("galgame_script".to_string()),
            scope_id: None,
            chapter_id: None,
            mode: Some(GALGAME_PIPELINE_MODE_FULL.to_string()),
            input_fingerprint_json: Some(fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(initial_stage.key().to_string()),
            active_task_id: input.task_id.clone(),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let result = state
        .run_pipeline(
            project.id,
            &prompt_renderer,
            GalgamePipelineRunOptions {
                workflow_run_id: workflow_run.id,
                task_id: input.task_id,
                progress_reporter,
                resume_from_checkpoint: input.resume_from_checkpoint,
                start_from_stage,
            },
        )
        .await;
    match result {
        Ok(detail) => {
            state
                .orchestrator
                .mark_workflow_status(workflow_run.id, WORKFLOW_STATUS_COMPLETED, false, None)
                .await
                .map_err(|err| err.to_string())?;
            Ok(detail)
        }
        Err(err) => {
            let cancelled = matches!(&err, AppError::Cancelled);
            let error_message = err.to_string();
            let status = if cancelled {
                WORKFLOW_STATUS_CANCELLED
            } else {
                WORKFLOW_STATUS_FAILED
            };
            let persisted_message = if cancelled {
                "Galgame 管线已取消"
            } else {
                &error_message
            };
            if let Err(status_error) = state
                .orchestrator
                .mark_workflow_status(workflow_run.id, status, true, Some(persisted_message))
                .await
            {
                return Err(workflow_status_write_error(&error_message, &status_error));
            }
            Err(error_message)
        }
    }
}
