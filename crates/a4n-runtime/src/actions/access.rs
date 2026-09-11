use crate::AppState;
use a4n_api_contract::collaboration::ProjectAccessRole;
use a4n_error::AppError;

pub(crate) async fn verify_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .project_access_application()
        .verify_permission(user_id, project_id, ProjectAccessRole::Viewer)
        .await
        .map(|_| ())
}

pub(crate) async fn verify_project_editor(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .project_access_application()
        .verify_permission(user_id, project_id, ProjectAccessRole::Editor)
        .await
        .map(|_| ())
}

pub(crate) async fn verify_project_editor_string(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    verify_project_editor(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn verify_project_owner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .project_access_application()
        .verify_permission(user_id, project_id, ProjectAccessRole::Owner)
        .await
        .map(|_| ())
}

pub(crate) async fn verify_project_owner_string(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    verify_project_owner(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())
}
