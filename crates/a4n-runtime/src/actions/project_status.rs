use crate::AppState;
use a4n_core::models::project::novel_project;
use a4n_domain::project::{ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary};

pub async fn get_project_status_machine(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectStatusMachineSummary, String> {
    state
        .orchestrator()
        .project_status_machine_for_user_validated(user_id, project_id)
        .await
        .map_err(|error| error.to_string())
}

pub async fn preview_rollback(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectRollbackPreview, String> {
    state
        .orchestrator()
        .preview_project_status_rollback_for_user_validated(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn forward_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator()
        .forward_project_status_for_user_validated(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn rollback_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<novel_project::Model, String> {
    state
        .orchestrator()
        .rollback_project_status_for_user_validated(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}
