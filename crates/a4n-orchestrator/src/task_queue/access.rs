use a4n_core::error::AppError;
use a4n_queue::priority::QueueStats;

use super::PersistentTaskQueue;

#[cfg(test)]
mod tests;

impl PersistentTaskQueue {
    pub async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        for cancelled_id in self.inner.cancel_task_now(task_id)? {
            self.update_status(&cancelled_id, "cancelled", None).await?;
        }
        Ok(())
    }

    pub fn queue_stats(&self) -> QueueStats {
        self.inner.queue_stats()
    }
}
