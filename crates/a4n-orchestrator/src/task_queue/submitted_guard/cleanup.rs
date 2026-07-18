use std::sync::Arc;

use a4n_db::SharedDatabaseConnection;
use a4n_queue::memory_queue::MemoryTaskQueue;

use super::super::queue_db_error;

pub(super) fn cleanup_dropped_task(
    inner: &Arc<MemoryTaskQueue>,
    db: &SharedDatabaseConnection,
    task_id: String,
) {
    let cancelled_ids = match inner.cancel_task_now(&task_id) {
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
    let db = db.clone();
    tokio::spawn(async move {
        for cancelled_id in cancelled_ids {
            if let Err(err) =
                a4n_db::queue::update_task_status(db.as_ref(), &cancelled_id, "cancelled", None)
                    .await
                    .map_err(queue_db_error)
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
