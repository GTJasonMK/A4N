use crate::{AppState, actions::admin_records};
use a4n_core::models::generation::workflow_run;
use a4n_db::workflow::WorkflowRunSummary;

use super::project::{project_visible_to_user, verify_task_project_access};

pub(in crate::actions::task_records) async fn task_scope_visible_to_user(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<bool, String> {
    if let Some(project_id) = project_id.or(root_project_id) {
        return project_visible_to_user(state, user_id, project_id).await;
    }
    admin_records::is_active_admin_user(state, user_id)
        .await
        .map_err(|err| err.to_string())
}

pub(in crate::actions::task_records) async fn workflow_run_summary_visible_to_user(
    state: &AppState,
    user_id: i64,
    run: &WorkflowRunSummary,
) -> Result<bool, String> {
    if let Some(project_id) = run.project_id.or(run.root_project_id) {
        return project_visible_to_user(state, user_id, project_id).await;
    }
    admin_records::is_active_admin_user(state, user_id)
        .await
        .map_err(|err| err.to_string())
}

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
    run: &workflow_run::Model,
) -> Result<(), String> {
    verify_task_project_access(state, user_id, run.project_id, run.root_project_id).await
}
