use std::sync::Arc;

use a4n_db::SharedDatabaseConnection;
use a4n_error::AppError;
use a4n_queue::memory_queue::MemoryTaskQueue;

use super::concurrency::QueueConcurrencyLimits;
use super::{PersistentTaskQueue, queue_db_error};
mod recovery;
mod status;
mod submission;

impl PersistentTaskQueue {
    /// 创建并恢复未完成任务
    pub async fn new(db: SharedDatabaseConnection) -> Result<Self, AppError> {
        let limits = QueueConcurrencyLimits::load(&db).await?;
        let inner = Arc::new(MemoryTaskQueue::with_limits(
            limits.llm,
            limits.image,
            limits.video,
            limits.embedding,
            limits.background,
        ));
        let queue = Self { inner, db };
        recovery::recover_pending(&queue).await?;
        Ok(queue)
    }
}
