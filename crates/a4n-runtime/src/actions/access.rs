use crate::AppState;
use a4n_core::error::AppError;
use a4n_core::models::project::novel_project;
use a4n_services::collaboration::ProjectAccessRole;
use a4n_services::project::PROJECT_KIND_TEXT;
use a4n_services::project_family::{ProjectFamilyContext, load_derived_project_context};
use sea_orm::EntityTrait;

pub(crate) async fn verify_project_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .collaboration
        .verify_permission(project_id, user_id, ProjectAccessRole::Viewer)
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
        .collaboration
        .verify_permission(project_id, user_id, ProjectAccessRole::Editor)
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

pub(crate) async fn verify_projects_editor_string(
    state: &AppState,
    user_id: i64,
    project_ids: &[i64],
) -> Result<(), String> {
    state
        .collaboration
        .verify_permissions_for_projects(project_ids, user_id, ProjectAccessRole::Editor)
        .await
        .map_err(|err| err.to_string())
}

pub(crate) async fn verify_project_owner(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    state
        .collaboration
        .verify_permission(project_id, user_id, ProjectAccessRole::Owner)
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

pub(crate) async fn verify_project_or_derived_family_access(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<(), AppError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
    if project.project_kind == PROJECT_KIND_TEXT {
        verify_project_access(state, user_id, project.id).await?;
        return Ok(());
    }
    verify_derived_project_family_context(state, user_id, project.id)
        .await
        .map(|_| ())
}

pub(crate) async fn verify_derived_project_family_context(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ProjectFamilyContext, AppError> {
    let context = load_derived_project_context(state.db.as_ref(), project_id).await?;
    verify_project_access(state, user_id, context.project.id).await?;
    verify_project_access(state, user_id, context.root_project.id).await?;
    Ok(context)
}
