use a4n_core::error::AppError;
use a4n_queue::priority::TaskInfo;

use super::super::PersistentTaskQueue;
use crate::workflow::workflow_read_app_error;

pub(super) async fn task_status(
    queue: &PersistentTaskQueue,
    task_id: &str,
) -> Result<TaskInfo, AppError> {
    a4n_db::workflow::persisted_task_by_id(queue.db.as_ref(), task_id)
        .await
        .map_err(workflow_read_app_error)?
        .map(|item| item.task)
        .ok_or_else(|| AppError::NotFound(format!("任务不存在: {task_id}")))
}

pub(super) async fn task_history(
    queue: &PersistentTaskQueue,
    project_id: Option<i64>,
    offset: u64,
    limit: u64,
) -> Result<Vec<TaskInfo>, AppError> {
    a4n_db::workflow::persisted_task_history(queue.db.as_ref(), project_id, offset, limit)
        .await
        .map_err(workflow_read_app_error)
        .map(|items| items.into_iter().map(|item| item.task).collect())
}
