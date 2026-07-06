use super::basic::project_summary_with_family_for_user;
use a4n_runtime::AppState;
use a4n_runtime::actions::project_duplication::DuplicateProjectInput;
use a4n_runtime::actions::project_records::ProjectSummary;
use tauri::State;

#[tauri::command]
pub async fn duplicate_project(
    state: State<'_, AppState>,
    user_id: i64,
    input: DuplicateProjectInput,
) -> Result<ProjectSummary, String> {
    let project =
        a4n_runtime::actions::project_duplication::duplicate_project(state.inner(), user_id, input)
            .await?;
    project_summary_with_family_for_user(&state, user_id, &project).await
}

#[tauri::command]
pub async fn branch_project(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    branch_title: String,
    at_status: String,
) -> Result<ProjectSummary, String> {
    let project = a4n_runtime::actions::project_duplication::branch_project(
        state.inner(),
        user_id,
        project_id,
        branch_title,
        at_status,
    )
    .await?;
    project_summary_with_family_for_user(&state, user_id, &project).await
}
