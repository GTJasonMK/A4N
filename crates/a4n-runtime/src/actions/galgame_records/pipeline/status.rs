use super::super::GalgameProjectInput;
use super::contracts::{
    GalgamePipelineStatusSummary, GalgameWorkflowCheckpointSummary, GalgameWorkflowRunSummary,
};
use crate::AppState;

pub async fn get_galgame_pipeline_status(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgamePipelineStatusSummary, String> {
    let status = state
        .orchestrator()
        .get_galgame_pipeline_status_for_user_validated(user_id, input.project_id)
        .await
        .map_err(|error| error.to_string())?;
    let checkpoints = status
        .checkpoints
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
        run: status.run.map(|run| GalgameWorkflowRunSummary {
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
