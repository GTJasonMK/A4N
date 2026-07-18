use crate::{AppState, actions::app_error_message};
use a4n_queue::priority::TaskQueue;

use super::super::{
    QueueStatsSummary, TaskInfoSummary,
    access::{require_admin_for_unscoped_task, verify_task_access, verify_task_history_access},
    projections::{
        queue_stats_summary, task_info_summary, validate_optional_project_id,
        validate_task_history_limit,
    },
};

pub async fn cancel_task(state: &AppState, user_id: i64, task_id: &str) -> Result<(), String> {
    verify_task_access(state, user_id, task_id).await?;
    state
        .task_queue
        .cancel(task_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn get_queue_stats(state: &AppState, user_id: i64) -> Result<QueueStatsSummary, String> {
    require_admin_for_unscoped_task(state, user_id).await?;
    Ok(queue_stats_summary(&state.task_queue.queue_stats()))
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
    let history = state
        .orchestrator
        .persisted_task_history(project_id, offset, limit)
        .await
        .map_err(app_error_message)?;
    Ok(history
        .iter()
        .map(|item| task_info_summary(&item.task))
        .collect())
}
