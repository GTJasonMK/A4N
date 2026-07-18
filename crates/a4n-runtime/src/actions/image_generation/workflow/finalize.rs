use super::super::GeneratedImageSummary;
use crate::AppState;
use a4n_orchestrator::workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED};

pub(crate) async fn finalize_single_image_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<GeneratedImageSummary, String>,
) -> Result<(), String> {
    finalize_image_workflow(
        state,
        workflow_run_id,
        result.as_ref().map(|_| ()).map_err(String::as_str),
    )
    .await
}

pub(crate) async fn finalize_batch_image_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: &Result<Vec<GeneratedImageSummary>, String>,
) -> Result<(), String> {
    finalize_image_workflow(
        state,
        workflow_run_id,
        result.as_ref().map(|_| ()).map_err(String::as_str),
    )
    .await
}

async fn finalize_image_workflow(
    state: &AppState,
    workflow_run_id: i64,
    result: Result<(), &str>,
) -> Result<(), String> {
    let (status, can_resume, error_message) = match result {
        Ok(_) => (WORKFLOW_STATUS_COMPLETED, false, None),
        Err(message) => (WORKFLOW_STATUS_FAILED, true, Some(message)),
    };
    state
        .orchestrator
        .mark_workflow_status(workflow_run_id, status, can_resume, error_message)
        .await
        .map(|_| ())
        .map_err(|err| err.to_string())
}
