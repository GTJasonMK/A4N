//! 任务队列 runtime 窄 facade。

use std::future::Future;

use a4n_core::error::AppError;
use a4n_orchestrator::{OrchestratorContext, task_queue::SubmittedTaskGuard};
use a4n_queue::{
    priority::{QueueStats, TaskKind, TaskPermit, TaskPriority},
    queue::{TaskFamilyContext, TaskWorkflowContext},
};
use tokio_util::sync::CancellationToken;

pub(crate) struct TaskQueueRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> TaskQueueRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn queue_run_tracked<F, Fut, T>(
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
        Box::pin(
            self.orchestrator
                .queue_run_tracked(task_type, priority, project_id, operation),
        )
        .await
    }

    pub(crate) async fn queue_run_tracked_with_cancel<F, Fut, T>(
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
        Box::pin(self.orchestrator.queue_run_tracked_with_cancel(
            task_type,
            priority,
            project_id,
            cancel_token,
            operation,
        ))
        .await
    }

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
        Box::pin(
            self.orchestrator
                .queue_run_tracked_with_family_context(task_type, priority, context, operation),
        )
        .await
    }

    pub(crate) async fn queue_run_tracked_with_workflow_context<F, Fut, T>(
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
        Box::pin(self.orchestrator.queue_run_tracked_with_workflow_context(
            task_type, priority, project_id, workflow, operation,
        ))
        .await
    }

    pub(crate) async fn queue_submit(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.orchestrator
            .queue_submit(task_type, priority, project_id)
            .await
    }

    pub(crate) async fn queue_submit_with_workflow_context(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
    ) -> Result<String, AppError> {
        self.orchestrator
            .queue_submit_with_workflow_context(task_type, priority, project_id, workflow)
            .await
    }

    pub(crate) fn queue_submitted_task_guard(&self, task_id: String) -> SubmittedTaskGuard {
        self.orchestrator.queue_submitted_task_guard(task_id)
    }

    pub(crate) async fn queue_acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.orchestrator
            .queue_acquire_task_with_cancel(task_id, cancel_token)
            .await
    }

    pub(crate) async fn queue_cancel(&self, task_id: &str) -> Result<(), AppError> {
        self.orchestrator.queue_cancel(task_id).await
    }

    pub(crate) async fn queue_mark_done(&self, task_id: &str) -> Result<(), AppError> {
        self.orchestrator.queue_mark_done(task_id).await
    }

    pub(crate) async fn queue_mark_failed(
        &self,
        task_id: &str,
        error: &str,
    ) -> Result<(), AppError> {
        self.orchestrator.queue_mark_failed(task_id, error).await
    }

    pub(crate) fn queue_stats(&self) -> QueueStats {
        self.orchestrator.queue_stats()
    }
}
