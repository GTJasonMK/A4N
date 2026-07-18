use super::super::{GalgameProjectInput, project_record_by_id};
use super::contracts::{
    GALGAME_PIPELINE_WORKFLOW_KIND, GalgamePipelineStatusSummary, GalgameWorkflowCheckpointSummary,
    GalgameWorkflowRunSummary,
};
use crate::AppState;
use crate::actions::access;
use a4n_orchestrator::galgame::PROJECT_KIND_GALGAME;
use a4n_orchestrator::workflow::ListWorkflowRunsInput;

pub async fn get_galgame_pipeline_status(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    access::verify_project_access_string(state, user_id, input.project_id).await?;
    let project = project_record_by_id(state, input.project_id).await?;
    if project.project_kind != PROJECT_KIND_GALGAME {
        return Err(format!("项目不是 Galgame 派生项目: {}", project.id));
    }
    let run = state
        .orchestrator
        .workflow_run_models(ListWorkflowRunsInput {
            project_id: Some(project.id),
            workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
            status: None,
            resumable_only: None,
            limit: Some(1),
        })
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .next();
    let Some(run) = run else {
        return Ok(GalgamePipelineStatusSummary {
            run: None,
            checkpoints: Vec::new(),
        });
    };
    let checkpoints = state
        .orchestrator
        .workflow_checkpoint_models(run.id)
        .await
        .map_err(|err| err.to_string())?
        .into_iter()
        .map(|checkpoint| GalgameWorkflowCheckpointSummary {
            id: checkpoint.id,
            step_key: checkpoint.step_key,
            step_order: checkpoint.step_order,
            status: checkpoint.status,
            checkpoint_json: checkpoint.checkpoint_json,
            artifacts_json: checkpoint.artifacts_json,
            error_message: checkpoint.error_message,
            created_at: checkpoint.created_at.to_string(),
            updated_at: checkpoint.updated_at.to_string(),
            completed_at: checkpoint.completed_at.map(|value| value.to_string()),
        })
        .collect();
    Ok(GalgamePipelineStatusSummary {
        run: Some(GalgameWorkflowRunSummary {
            id: run.id,
            status: run.status,
            mode: run.mode,
            current_step_key: run.current_step_key,
            progress_json: run.progress_json,
            active_task_id: run.active_task_id,
            can_resume: run.can_resume,
            resume_block_reason: run.resume_block_reason,
            error_message: run.error_message,
            created_at: run.created_at.to_string(),
            updated_at: run.updated_at.to_string(),
            completed_at: run.completed_at.map(|value| value.to_string()),
        }),
        checkpoints,
    })
}
