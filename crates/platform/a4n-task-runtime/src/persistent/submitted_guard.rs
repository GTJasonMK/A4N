use std::sync::Arc;

use a4n_db::DbHandle;
use a4n_queue::memory_queue::MemoryTaskQueue;

mod cleanup;
#[cfg(test)]
mod tests;

use cleanup::cleanup_dropped_task;
pub(in crate::persistent) use cleanup::persist_dropped_status;

pub struct SubmittedTaskGuard {
    inner: Arc<MemoryTaskQueue>,
    db: DbHandle,
    task_id: Option<String>,
}

impl SubmittedTaskGuard {
    pub(super) fn new(inner: Arc<MemoryTaskQueue>, db: DbHandle, task_id: String) -> Self {
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
        cleanup_dropped_task(&self.inner, &self.db, task_id);
    }
}
