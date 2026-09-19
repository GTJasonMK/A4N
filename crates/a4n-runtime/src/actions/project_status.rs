use crate::AppState;
use a4n_api_contract::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary, ProjectSummary,
};

pub async fn get_project_status_machine(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectStatusMachineSummary, String> {
    state
        .project_status_read_application()
        .status_machine(user_id, project_id)
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
        .project_status_read_application()
        .preview_rollback(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn forward_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    state
        .project_status_mutation_application()
        .forward_status(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}

pub async fn rollback_project_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    state
        .project_status_mutation_application()
        .rollback_status(user_id, project_id, target)
        .await
        .map_err(|error| error.to_string())
}

/// Existing command modules still use these names; both now delegate to the
/// same project status port and contain no separate projection logic.
pub async fn forward_project_status_summary(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    forward_project_status(state, user_id, project_id, target).await
}

pub async fn rollback_project_status_summary(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    rollback_project_status(state, user_id, project_id, target).await
}
