use crate::AppState;
use a4n_api_contract::workflow::{WorkflowResumeRecord, WorkflowRunSummary};

use super::project::verify_task_project_access;

pub(in crate::actions::task_records) async fn verify_workflow_run_summary_access(
    state: &AppState,
    user_id: i64,
    run: &WorkflowRunSummary,
) -> Result<(), String> {
    verify_task_project_access(state, user_id, run.project_id, run.root_project_id).await
}

pub(in crate::actions::task_records) async fn verify_workflow_run_access(
    state: &AppState,
    user_id: i64,
    run: &WorkflowResumeRecord,
) -> Result<(), String> {
    verify_task_project_access(state, user_id, run.project_id, run.root_project_id).await
}
