use a4n_core::error::AppError;
use a4n_db::queue::RecoverableQueuedTask;

use super::super::{PersistentTaskQueue, queue_db_error};
use crate::workflow::workflow_write_app_error;

pub(super) async fn recover_task(
    queue: &PersistentTaskQueue,
    task: RecoverableQueuedTask,
) -> Result<(), AppError> {
    a4n_db::queue::update_task_status(
        queue.db.as_ref(),
        &task.id,
        "interrupted",
        Some("应用重启后中断未绑定执行器的队列任务"),
    )
    .await
    .map_err(queue_db_error)?;
    if let Some(workflow_run_id) = task.workflow_run_id {
        a4n_db::workflow::mark_recovered_task_workflow_interrupted(
            queue.db.as_ref(),
            workflow_run_id,
            &task.id,
        )
        .await
        .map_err(workflow_write_app_error)?;
    }
    Ok(())
}
