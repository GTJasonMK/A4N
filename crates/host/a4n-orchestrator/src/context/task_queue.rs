use std::future::Future;

use a4n_error::AppError;
use a4n_queue::{
    TaskFamilyContext,
    priority::{QueueStats, TaskKind, TaskPriority},
};

use super::OrchestratorContext;

impl OrchestratorContext {
    pub(crate) async fn queue_run_tracked_with_family_context<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        context: TaskFamilyContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.task_queue
            .run_tracked_with_family_context(task_type, priority, context, operation)
            .await
    }

    pub(crate) async fn queue_cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.task_queue.cancel(task_id).await
    }

    pub(crate) fn queue_stats(&self) -> QueueStats {
        self.task_queue.queue_stats()
    }
}

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
