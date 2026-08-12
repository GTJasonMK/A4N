use crate::{OrchestratorContext, workflow::WorkflowRunFinalizeOutcome};
use a4n_core::error::AppError;

pub(in crate::scene_profile::application) async fn finalize_scene_profile_sync_workflow(
    state: &OrchestratorContext,
    workflow_run_id: i64,
    result: &Result<(), String>,
) -> Result<(), AppError> {
    let (outcome, error_message) = match result {
        Ok(_) => (WorkflowRunFinalizeOutcome::Completed, None),
        Err(message) if message == "场景档案同步已停止" => (
            WorkflowRunFinalizeOutcome::Cancelled,
            Some(message.as_str()),
        ),
        Err(message) => (WorkflowRunFinalizeOutcome::Failed, Some(message.as_str())),
    };
    state
        .finalize_workflow_run_outcome_validated(workflow_run_id, outcome, error_message)
        .await?;
    Ok(())
}
