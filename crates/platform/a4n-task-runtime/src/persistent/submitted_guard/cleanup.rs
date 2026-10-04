use std::sync::Arc;

use a4n_db::DbHandle;
use a4n_queue::memory_queue::MemoryTaskQueue;

use super::super::queue_db_error;

pub(super) fn cleanup_dropped_task(inner: &Arc<MemoryTaskQueue>, db: &DbHandle, task_id: String) {
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
    persist_dropped_statuses(db.clone(), cancelled_ids, "cancelled", None);
}

pub(in crate::persistent) fn persist_dropped_status(
    db: DbHandle,
    task_id: String,
    status: &'static str,
    error: Option<&'static str>,
) {
    persist_dropped_statuses(db, vec![task_id], status, error);
}

fn persist_dropped_statuses(
    db: DbHandle,
    task_ids: Vec<String>,
    status: &'static str,
    error: Option<&'static str>,
) {
    tokio::spawn(async move {
        for task_id in task_ids {
            if let Err(err) = a4n_db::queue::update_task_status(&db, &task_id, status, error)
                .await
                .map_err(queue_db_error)
            {
                tracing::warn!(
                    task_id = %task_id,
                    error = %err,
                    "异步同步被 drop 的队列任务状态失败"
                );
            }
        }
    });
}
