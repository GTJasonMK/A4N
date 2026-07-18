use a4n_core::error::AppError;
use a4n_queue::priority::{QueueStats, TaskInfo, TaskPermit, TaskPriority, TaskQueue, TaskType};
use async_trait::async_trait;
use tokio_util::sync::CancellationToken;

use self::cancellation::cancel_task;
use self::reads::{task_history, task_status};
use super::PersistentTaskQueue;

mod cancellation;
mod reads;

#[async_trait]
impl TaskQueue for PersistentTaskQueue {
    async fn submit(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.submit_internal(task_type, priority, project_id, None, None)
            .await
    }

    async fn acquire(&self, task_type: TaskType) -> Result<TaskPermit, AppError> {
        let permit = self.inner.acquire(task_type).await?;
        self.mark_permit_running(permit).await
    }

    async fn acquire_with_cancel(
        &self,
        task_type: TaskType,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        match self
            .inner
            .acquire_with_cancel(task_type, cancel_token)
            .await?
        {
            Some(permit) => Ok(Some(self.mark_permit_running(permit).await?)),
            None => Ok(None),
        }
    }

    async fn status(&self, task_id: &str) -> Result<TaskInfo, AppError> {
        task_status(self, task_id).await
    }

    async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        cancel_task(self, task_id).await
    }

    fn queue_stats(&self) -> QueueStats {
        self.inner.queue_stats()
    }

    async fn history(
        &self,
        project_id: Option<i64>,
        offset: u64,
        limit: u64,
    ) -> Result<Vec<TaskInfo>, AppError> {
        task_history(self, project_id, offset, limit).await
    }
}
