use crate::AppState;
use a4n_api_contract::task::{QueueStatsSummary, TaskInfoSummary};
use a4n_domain::workflow::{validate_optional_project_id, validate_task_history_limit};
use a4n_task_runtime::channel::normalize_task_id;

use super::super::access::{
    require_admin_for_unscoped_task, verify_task_access, verify_task_history_access,
};

pub async fn cancel_task(state: &AppState, user_id: i64, task_id: &str) -> Result<bool, String> {
    let task_id = normalize_task_id(task_id).map_err(|error| error.to_string())?;
    verify_task_access(state, user_id, &task_id).await?;
    state
        .task_cancellation_application()
        .cancel(&task_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_queue_stats(state: &AppState, user_id: i64) -> Result<QueueStatsSummary, String> {
    require_admin_for_unscoped_task(state, user_id).await?;
    state
        .task_queue_stats_application()
        .stats()
        .await
        .map_err(|error| error.to_string())
}

pub async fn get_task_history(
    state: &AppState,
    user_id: i64,
    project_id: Option<i64>,
    offset: u64,
    limit: u64,
) -> Result<Vec<TaskInfoSummary>, String> {
    validate_task_history_limit(limit)?;
    validate_optional_project_id(project_id)?;
    verify_task_history_access(state, user_id, project_id).await?;
    state
        .task_read_application()
        .history(project_id, offset, limit)
        .await
        .map_err(|error| error.to_string())
}
