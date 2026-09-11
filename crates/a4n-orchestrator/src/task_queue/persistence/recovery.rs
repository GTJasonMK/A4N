use a4n_error::AppError;

use super::{PersistentTaskQueue, queue_db_error};
use crate::workflow::workflow_write_app_error;

#[cfg(test)]
mod tests;

pub(super) async fn recover_pending(queue: &PersistentTaskQueue) -> Result<(), AppError> {
    let active_task_ids = a4n_db::queue::recoverable_active_task_ids(queue.db.as_ref())
        .await
        .map_err(queue_db_error)?;
    a4n_db::queue::interrupt_active_tasks(
        queue.db.as_ref(),
        &active_task_ids,
        "应用重启后中断未绑定执行器的队列任务",
    )
    .await
    .map_err(queue_db_error)?;
    a4n_db::workflow::recover_stale_in_process_workflows(queue.db.as_ref())
        .await
        .map_err(workflow_write_app_error)?;
    a4n_db::video::recover_stale_generated_videos_without_provider_job(queue.db.as_ref())
        .await
        .map_err(crate::video::video_db_error)?;
    Ok(())
}
