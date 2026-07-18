use crate::{
    AppState,
    actions::{
        access,
        admin_records::{self, AdminPermission},
    },
};
use a4n_core::error::AppError;

pub(super) async fn project_visible_to_user(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<bool, String> {
    match access::verify_project_editor(state, user_id, project_id).await {
        Ok(()) => Ok(true),
        Err(AppError::Forbidden(_)) | Err(AppError::NotFound(_)) => Ok(false),
        Err(err) => Err(err.to_string()),
    }
}

pub(super) async fn verify_task_project_access(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
) -> Result<(), String> {
    if let Some(project_id) = project_id.or(root_project_id) {
        return access::verify_project_editor_string(state, user_id, project_id).await;
    }
    require_admin_for_unscoped_task(state, user_id).await
}

pub(in crate::actions::task_records) async fn require_admin_for_unscoped_task(
    state: &AppState,
    user_id: i64,
) -> Result<(), String> {
    admin_records::require_admin_permission(state, user_id, AdminPermission::ManageSettings)
        .await
        .map_err(|err| err.to_string())
}
