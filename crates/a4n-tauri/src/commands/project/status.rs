use super::basic::project_summary_with_family_for_user;
use a4n_api_contract::project::{
    ProjectRollbackPreview, ProjectStatus, ProjectStatusMachineSummary,
};
use a4n_runtime::AppState;
use a4n_runtime::host_api::project_records::ProjectSummary;
use a4n_runtime::host_api::project_status as project_status_actions;
use tauri::State;

#[tauri::command]
pub async fn forward_project_status(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    let project =
        project_status_actions::forward_project_status(&state, user_id, project_id, target).await?;

    project_summary_with_family_for_user(&state, user_id, &project).await
}

#[tauri::command]
pub async fn rollback_project_status(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectSummary, String> {
    let project =
        project_status_actions::rollback_project_status(&state, user_id, project_id, target)
            .await?;

    project_summary_with_family_for_user(&state, user_id, &project).await
}

#[tauri::command]
pub async fn get_project_status_machine(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectStatusMachineSummary, String> {
    project_status_actions::get_project_status_machine(&state, user_id, project_id).await
}

#[tauri::command]
pub async fn preview_rollback(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    target: ProjectStatus,
) -> Result<ProjectRollbackPreview, String> {
    project_status_actions::preview_rollback(&state, user_id, project_id, target).await
}
