use crate::{
    AppState,
    actions::{access, app_error_message},
};
use a4n_task_runtime::channel::TaskHandle;

use super::project::{require_admin_for_unscoped_task, verify_task_project_access};

pub(in crate::actions::task_records) async fn verify_task_history_access(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
) -> Result<(), String> {
    if let Some(project_id) = project_id {
        return access::verify_project_editor_string(state, user_id, project_id).await;
    }
    require_admin_for_unscoped_task(state, user_id).await
}

pub(in crate::actions::task_records) async fn verify_task_access(
    state: &AppState,
    user_id: i64,
    task_id: &str,
) -> Result<(), String> {
    if let Some(task) = state
        .workflow_task_lookup_application()
        .task_by_id(task_id)
        .await
        .map_err(app_error_message)?
    {
        return verify_task_project_access(state, user_id, task.project_id, task.root_project_id)
            .await;
    }
    let handle = state
        .task_manager()
        .get(task_id)
        .ok_or_else(|| format!("任务不存在: {task_id}"))?;
    verify_task_handle_access(state, user_id, &handle).await
}

pub(in crate::actions::task_records) async fn verify_task_handle_access(
    state: &AppState,
    user_id: i64,
    handle: &TaskHandle,
) -> Result<(), String> {
    if handle.project_id.is_none()
        && handle.root_project_id.is_none()
        && handle.owner_user_id == Some(user_id)
    {
        return Ok(());
    }
    verify_task_project_access(state, user_id, handle.project_id, handle.root_project_id).await
}
