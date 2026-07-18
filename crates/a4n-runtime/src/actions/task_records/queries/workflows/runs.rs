use crate::{
    AppState,
    actions::{access, app_error_message},
};

use super::super::super::{
    GetWorkflowRunInput, ListWorkflowRunsInput, WorkflowRunDetailSummary, WorkflowRunSummary,
    access::{verify_workflow_run_summary_access, workflow_run_summary_visible_to_user},
    projections::validate_optional_project_id,
};

pub async fn list_workflow_runs_for_user(
    state: &AppState,
    user_id: i64,
    input: ListWorkflowRunsInput,
) -> Result<Vec<WorkflowRunSummary>, String> {
    validate_optional_project_id(input.project_id)?;
    if let Some(project_id) = input.project_id {
        access::verify_project_editor_string(state, user_id, project_id).await?;
    }
    let runs = state
        .orchestrator
        .workflow_runs(input)
        .await
        .map_err(app_error_message)?;
    let mut visible = Vec::new();
    for run in runs {
        if workflow_run_summary_visible_to_user(state, user_id, &run).await? {
            visible.push(run);
        }
    }
    Ok(visible)
}

pub async fn get_workflow_run_detail_for_user(
    state: &AppState,
    user_id: i64,
    input: GetWorkflowRunInput,
) -> Result<WorkflowRunDetailSummary, String> {
    let detail = state
        .orchestrator
        .workflow_run_detail(input.workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &detail.run).await?;
    Ok(detail)
}
