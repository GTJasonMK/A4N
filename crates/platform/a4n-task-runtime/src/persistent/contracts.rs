use std::sync::Arc;

use a4n_db::DbHandle;
use a4n_queue::memory_queue::MemoryTaskQueue;

mod tracked;

pub(in crate::persistent) use tracked::RunTrackedTaskInput;

pub struct PersistentTaskQueue {
    pub(in crate::persistent) inner: Arc<MemoryTaskQueue>,
    pub(in crate::persistent) db: DbHandle,
}
