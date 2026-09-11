use crate::{
    AppState,
    actions::{
        access,
        admin_records::{self, AdminPermission},
    },
};
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;
use std::collections::HashSet;

pub(in crate::actions::task_records) async fn project_ids_visible_to_user(
    state: &AppState,
    user_id: i64,
    project_ids: &[i64],
) -> Result<HashSet<i64>, String> {
    let project_ids = project_ids
        .iter()
        .copied()
        .filter(|project_id| *project_id > 0)
        .collect::<Vec<_>>();
    if project_ids.is_empty() {
        return Ok(HashSet::new());
    }
    match state
        .project_access_application()
        .project_ids_with_permission(user_id, &project_ids, ProjectAccessRole::Editor)
        .await
    {
        Ok(project_ids) => Ok(project_ids.into_iter().collect()),
        Err(AppError::Forbidden(_)) | Err(AppError::NotFound(_)) => Ok(HashSet::new()),
        Err(err) => Err(err.to_string()),
    }
}

pub(in crate::actions::task_records) async fn verify_task_project_access(
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
