use crate::state::AppState;
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};
use serde_json::Value;

pub(crate) async fn finalize_scene_profile_sync_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Value, String>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message.as_str())),
    };
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map_err(|err| err.to_string())?;
    Ok(())
}
