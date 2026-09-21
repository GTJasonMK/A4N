use crate::AppState;
use a4n_api_contract::workflow::{
    GetWorkflowRunInput, ListWorkflowRunsInput, WorkflowRunDetailSummary, WorkflowRunSummary,
};

pub async fn list_workflow_runs_for_user(
    state: &AppState,
    user_id: i64,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, String> {
    let runs = state
        .workflow_read_application()
        .list_for_user(user_id, input)
        .await
        .map_err(|error| error.to_string())?;
    Ok(runs)
}

pub async fn get_workflow_run_detail_for_user(
    state: &AppState,
    user_id: i64,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, String> {
    state
        .workflow_read_application()
        .detail_for_user(user_id, input)
        .await
        .map_err(|error| error.to_string())
}
