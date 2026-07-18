use std::future::Future;

use a4n_core::error::AppError;
use a4n_queue::{
    priority::{TaskPriority, TaskType},
    queue::{TaskFamilyContext, TaskWorkflowContext},
};
use tokio_util::sync::CancellationToken;

use super::super::PersistentTaskQueue;

impl PersistentTaskQueue {
    pub async fn run_tracked<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_with_cancel(
            task_type,
            priority,
            project_id,
            CancellationToken::new(),
            operation,
        )
        .await
    }

    pub async fn run_tracked_with_family_context<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        context: TaskFamilyContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_with_family_context_and_cancel(
            task_type,
            priority,
            context,
            CancellationToken::new(),
            operation,
        )
        .await
    }

    pub async fn run_tracked_with_workflow_context<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_with_workflow_context_and_cancel(
            task_type,
            priority,
            project_id,
            workflow,
            CancellationToken::new(),
            operation,
        )
        .await
    }
}
