use a4n_error::AppError;
use a4n_queue::priority::{QueueStats, TaskStatus};

use super::{PersistentTaskQueue, queue_db_error};

#[cfg(test)]
mod llm_tests;
#[cfg(test)]
mod tests;

impl PersistentTaskQueue {
    pub async fn cancel_workflow_llm_calls(&self, workflow_run_id: i64) -> Result<(), AppError> {
        let ids = a4n_db::queue::active_workflow_llm_task_ids(self.db.as_ref(), workflow_run_id)
            .await
            .map_err(queue_db_error)?;
        for id in ids {
            self.cancel_persisted_task(&id).await?;
        }
        Ok(())
    }

    pub async fn cancel(&self, task_id: &str) -> Result<bool, AppError> {
        self.cancel_persisted_task(task_id).await
    }

    /// The durable transition decides whether cancellation won a terminal race.
    pub async fn cancel_persisted_task(&self, task_id: &str) -> Result<bool, AppError> {
        let cancelled = a4n_db::queue::update_task_status(
            self.db.as_ref(),
            task_id,
            "cancelled",
            Some("用户取消持久化队列任务"),
        )
        .await
        .map_err(queue_db_error)?;
        if cancelled {
            match self
                .inner
                .project_task_terminal(task_id, TaskStatus::Cancelled)
            {
                Ok(()) | Err(AppError::NotFound(_)) => {}
                Err(error) => return Err(error),
            }
        }
        Ok(cancelled)
    }

    pub fn queue_stats(&self) -> QueueStats {
        self.inner.queue_stats()
    }
}
