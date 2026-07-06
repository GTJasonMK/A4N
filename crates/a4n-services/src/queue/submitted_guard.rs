use std::sync::Arc;

use a4n_engine::queue::memory_queue::MemoryTaskQueue;
use sea_orm::DatabaseConnection;

use super::status_store::update_task_status_by_db;

pub struct SubmittedTaskGuard {
    inner: Arc<MemoryTaskQueue>,
    db: Arc<DatabaseConnection>,
    task_id: Option<String>,
}

impl SubmittedTaskGuard {
    pub(super) fn new(
        inner: Arc<MemoryTaskQueue>,
        db: Arc<DatabaseConnection>,
        task_id: String,
    ) -> Self {
        Self {
            inner,
            db,
            task_id: Some(task_id),
        }
    }

    pub fn disarm(&mut self) {
        self.task_id = None;
    }
}

impl Drop for SubmittedTaskGuard {
    fn drop(&mut self) {
        let Some(task_id) = self.task_id.take() else {
            return;
        };
        let cancelled_ids = match self.inner.cancel_task_now(&task_id) {
            Ok(cancelled_ids) => cancelled_ids,
            Err(err) => {
                tracing::warn!(
                    task_id = %task_id,
                    error = %err,
                    "同步取消被 drop 的队列任务失败"
                );
                return;
            }
        };
        let db = self.db.clone();
        tokio::spawn(async move {
            for cancelled_id in cancelled_ids {
                if let Err(err) =
                    update_task_status_by_db(db.clone(), cancelled_id.clone(), "cancelled", None)
                        .await
                {
                    tracing::warn!(
                        task_id = %cancelled_id,
                        error = %err,
                        "异步同步被 drop 的队列任务取消状态失败"
                    );
                }
            }
        });
    }
}
