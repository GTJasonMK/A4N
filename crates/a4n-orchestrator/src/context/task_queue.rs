use std::future::Future;

use a4n_error::AppError;
use a4n_queue::{
    TaskFamilyContext, TaskWorkflowContext,
    priority::{TaskKind, TaskPriority},
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
}
