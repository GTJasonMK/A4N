use a4n_core::error::AppError;

use super::{PersistentTaskQueue, queue_db_error};
use crate::workflow::workflow_write_app_error;

mod task;
#[cfg(test)]
mod tests;

use task::recover_task;

pub(super) async fn recover_pending(queue: &PersistentTaskQueue) -> Result<(), AppError> {
    let active_tasks = a4n_db::queue::recoverable_active_tasks(queue.db.as_ref())
        .await
        .map_err(queue_db_error)?;
    for task in active_tasks {
        recover_task(queue, task).await?;
    }
    a4n_db::workflow::recover_stale_in_process_workflows(queue.db.as_ref())
        .await
        .map_err(workflow_write_app_error)?;
    a4n_db::video::recover_stale_generated_videos_without_provider_job(queue.db.as_ref())
        .await
        .map_err(crate::video::video_db_error)?;
    Ok(())
}
