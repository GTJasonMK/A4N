use a4n_core::error::AppError;

use super::super::super::PersistentTaskQueue;
use crate::workflow::workflow_write_app_error;

pub(super) async fn sync_workflow_active_task(
    queue: &PersistentTaskQueue,
    task_id: &str,
    status: &str,
) -> Result<(), AppError> {
    a4n_db::workflow::sync_workflow_active_task_for_queue_status(queue.db.as_ref(), task_id, status)
        .await
        .map_err(workflow_write_app_error)
}
