use std::sync::Arc;

use a4n_api_contract::task::{
    QueueSlotStatsSummary, QueueStatsSummary, TaskCancellationPort, TaskQueueStatsApplicationPort,
};
use a4n_error::AppError;
use a4n_task_runtime::persistent::PersistentTaskQueue;

pub(crate) struct TaskQueueApplication {
    queue: Arc<PersistentTaskQueue>,
}

impl TaskQueueApplication {
    pub(crate) fn new(queue: Arc<PersistentTaskQueue>) -> Self {
        Self { queue }
    }
}

#[async_trait::async_trait]
impl TaskCancellationPort for TaskQueueApplication {
    async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.queue.cancel(task_id).await
    }
}

#[async_trait::async_trait]
impl TaskQueueStatsApplicationPort for TaskQueueApplication {
    async fn stats(&self) -> Result<QueueStatsSummary, AppError> {
        let stats = self.queue.queue_stats();
        Ok(QueueStatsSummary {
            llm_queue: queue_slot_stats_summary(&stats.llm_queue),
            image_queue: queue_slot_stats_summary(&stats.image_queue),
            video_queue: queue_slot_stats_summary(&stats.video_queue),
            embedding_queue: queue_slot_stats_summary(&stats.embedding_queue),
            background_queue: queue_slot_stats_summary(&stats.background_queue),
            high_priority_waiting: stats.high_priority_waiting,
            medium_priority_waiting: stats.medium_priority_waiting,
            low_priority_waiting: stats.low_priority_waiting,
        })
    }
}

fn queue_slot_stats_summary(stats: &a4n_queue::priority::QueueSlotStats) -> QueueSlotStatsSummary {
    QueueSlotStatsSummary {
        running: stats.running,
        waiting: stats.waiting,
    }
}
