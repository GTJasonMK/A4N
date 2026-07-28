use crate::{
    AppState,
    actions::{access, app_error_message},
};

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
        .workflow()
        .persisted_task_by_id(task_id)
        .await
        .map_err(app_error_message)?
    {
        return verify_task_project_access(
            state,
            user_id,
            task.task.project_id,
            task.task.root_project_id,
        )
        .await;
    }
    let snapshot = state
        .active_task_snapshots()
        .into_iter()
        .find(|snapshot| snapshot.task_id == task_id)
        .ok_or_else(|| format!("任务不存在: {task_id}"))?;
    verify_task_project_access(
        state,
        user_id,
        snapshot.project_id,
        snapshot.root_project_id,
    )
    .await
}
