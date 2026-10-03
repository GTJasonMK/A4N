use std::future::Future;

use a4n_error::AppError;
use a4n_queue::{
    TaskFamilyContext,
    priority::{QueueStats, TaskKind, TaskPriority},
};

use super::OrchestratorContext;

mod ports;

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

    pub(crate) async fn queue_cancel(&self, task_id: &str) -> Result<bool, AppError> {
        self.task_queue.cancel(task_id).await
    }

    pub(crate) fn queue_stats(&self) -> QueueStats {
        self.task_queue.queue_stats()
    }
}
