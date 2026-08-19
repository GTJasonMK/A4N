use std::future::Future;

use a4n_core::error::AppError;
use a4n_queue::{
    TaskFamilyContext, TaskWorkflowContext,
    priority::{TaskKind, TaskPriority},
};
use tokio_util::sync::CancellationToken;

use super::super::{PersistentTaskQueue, RunTrackedTaskInput};

impl PersistentTaskQueue {
    pub(super) async fn run_tracked_with_family_context_and_cancel<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        context: TaskFamilyContext,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_internal(
            RunTrackedTaskInput {
                task_type,
                priority,
                project_id: Some(context.project_id),
                context: Some(context),
                workflow: None,
                cancel_token,
            },
            operation,
        )
        .await
    }

    pub(super) async fn run_tracked_with_workflow_context_and_cancel<F, Fut, T>(
        &self,
        task_type: TaskKind,
        priority: TaskPriority,
        project_id: Option<i64>,
        workflow: TaskWorkflowContext,
        cancel_token: CancellationToken,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        self.run_tracked_internal(
            RunTrackedTaskInput {
                task_type,
                priority,
                project_id,
                context: None,
                workflow: Some(workflow),
                cancel_token,
            },
            operation,
        )
        .await
    }
}
