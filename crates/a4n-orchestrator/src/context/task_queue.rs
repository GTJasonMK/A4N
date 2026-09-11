use std::future::Future;

use a4n_error::AppError;
use a4n_queue::{
    TaskFamilyContext, TaskWorkflowContext,
    priority::{QueueStats, TaskKind, TaskPriority},
};
use tokio_util::sync::CancellationToken;

use super::OrchestratorContext;
use crate::task_queue::RunningTask;

impl OrchestratorContext {
    pub async fn queue_start_task_with_cancel(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: Option<TaskWorkflowContext>,
        cancel_token: CancellationToken,
    ) -> Result<RunningTask, AppError> {
        self.task_queue
            .start_task_with_cancel(task_type, priority, project_id, workflow, cancel_token)
            .await
    }

    pub async fn queue_run_tracked<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.task_queue
            .run_tracked(task_type, priority, project_id, operation)
            .await
    }

    pub async fn queue_run_tracked_with_cancel<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.task_queue
            .run_tracked_with_cancel(task_type, priority, project_id, cancel_token, operation)
            .await
    }

    pub async fn queue_run_tracked_with_family_context<F, Fut, T>(
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

    pub async fn queue_run_tracked_with_workflow_context<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.task_queue
            .run_tracked_with_workflow_context(task_type, priority, project_id, workflow, operation)
            .await
    }

    pub async fn queue_cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.task_queue.cancel(task_id).await
    }

    pub fn queue_stats(&self) -> QueueStats {
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
