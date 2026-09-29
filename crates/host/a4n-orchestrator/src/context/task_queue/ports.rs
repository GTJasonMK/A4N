use a4n_error::AppError;

use crate::OrchestratorContext;

#[async_trait::async_trait]
impl a4n_api_contract::task::TaskQueueStatsApplicationPort for OrchestratorContext {
    async fn stats(&self) -> Result<a4n_api_contract::task::QueueStatsSummary, AppError> {
        let stats = OrchestratorContext::queue_stats(self);
        Ok(a4n_api_contract::task::QueueStatsSummary {
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

#[async_trait::async_trait]
impl a4n_api_contract::task::TaskCancellationPort for OrchestratorContext {
    async fn cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.queue_cancel(task_id).await
    }
}

fn queue_slot_stats_summary(
    stats: &a4n_queue::priority::QueueSlotStats,
) -> a4n_api_contract::task::QueueSlotStatsSummary {
    a4n_api_contract::task::QueueSlotStatsSummary {
        running: stats.running,
        waiting: stats.waiting,
    }
}
