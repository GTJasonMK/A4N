use a4n_error::AppError;
use a4n_queue::priority::QueueStats;

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

    pub async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.cancel_persisted_task(task_id).await
    }

    /// 内存终态可能已淘汰或随重启释放；数据库仍是持久任务终态的唯一来源。
    pub async fn cancel_persisted_task(&self, task_id: &str) -> Result<(), AppError> {
        match self.inner.cancel_task_now(task_id) {
            Ok(cancelled_ids) => {
                for cancelled_id in cancelled_ids {
                    self.update_status(&cancelled_id, "cancelled", None).await?;
                }
                Ok(())
            }
            Err(AppError::NotFound(_)) => a4n_db::queue::update_task_status(
                self.db.as_ref(),
                task_id,
                "cancelled",
                Some("取消未绑定执行器的持久化队列任务"),
            )
            .await
            .map_err(queue_db_error),
            Err(error) => Err(error),
        }
    }

    pub fn queue_stats(&self) -> QueueStats {
        self.inner.queue_stats()
    }
}
