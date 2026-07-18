use crate::state::AppState;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_CANCELLED;

pub(in crate::actions::canvas_workflow) async fn canvas_workflow_run_is_cancelled(
    state: &AppState,
    workflow_run_id: i64,
) -> Result<bool, String> {
    let run = state
        .orchestrator
        .workflow_run_model_by_id(workflow_run_id)
        .await
        .map_err(|err| err.to_string())?;
    Ok(run.status == WORKFLOW_STATUS_CANCELLED)
}
