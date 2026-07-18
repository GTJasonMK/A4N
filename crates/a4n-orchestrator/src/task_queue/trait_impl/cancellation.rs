use a4n_core::error::AppError;

use super::super::PersistentTaskQueue;

#[cfg(test)]
mod tests;

pub(super) async fn cancel_task(
    queue: &PersistentTaskQueue,
    task_id: &str,
) -> Result<(), AppError> {
    for cancelled_id in queue.inner.cancel_task_now(task_id)? {
        queue
            .update_status(&cancelled_id, "cancelled", None)
            .await?;
    }
    Ok(())
}
