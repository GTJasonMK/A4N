use crate::AppState;
use a4n_core::error::AppError;
use a4n_orchestrator::{collaboration::ProjectAccessRole, project::PROJECT_KIND_TEXT};

pub(crate) async fn verify_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .collaboration()
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Viewer)
        .await
        .map(|_| ())
}

pub(crate) async fn verify_project_access_string(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), String> {
    verify_project_access(state, user_id, project_id)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn verify_project_editor(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .collaboration()
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
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
        .collaboration()
        .verify_project_permission(project_id, user_id, ProjectAccessRole::Owner)
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

pub(crate) async fn verify_project_or_derived_family_editor(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    let project_kind = state.project_read().project_kind(project_id).await?;
    if project_kind == PROJECT_KIND_TEXT {
        verify_project_editor(state, user_id, project_id).await?;
        return Ok(());
    }
    let context = state
        .project_family()
        .derived_project_family_context(project_id)
        .await?;
    verify_project_editor(state, user_id, context.project.id).await?;
    verify_project_editor(state, user_id, context.root_project.id).await
}
