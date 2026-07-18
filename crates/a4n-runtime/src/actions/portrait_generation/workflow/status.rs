use crate::AppState;
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};

pub(crate) async fn finalize_portrait_workflow<T>(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<T, String>,
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
