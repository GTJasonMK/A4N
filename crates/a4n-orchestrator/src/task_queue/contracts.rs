use std::sync::Arc;

use a4n_db::SharedDatabaseConnection;
use a4n_queue::memory_queue::MemoryTaskQueue;

mod tracked;

pub(in crate::task_queue) use tracked::RunTrackedTaskInput;

pub struct PersistentTaskQueue {
    pub(in crate::task_queue) inner: Arc<MemoryTaskQueue>,
    pub(in crate::task_queue) db: SharedDatabaseConnection,
}
