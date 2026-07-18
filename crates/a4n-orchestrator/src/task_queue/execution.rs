use std::future::Future;

use a4n_core::error::AppError;
use a4n_queue::priority::{TaskPriority, TaskType};
use tokio_util::sync::CancellationToken;

use super::{PersistentTaskQueue, RunTrackedTaskInput, SubmittedTaskGuard};

mod cancellation;
mod contexts;
mod entrypoints;
mod lifecycle;
mod outcome;

impl PersistentTaskQueue {
    pub async fn run_tracked_with_cancel<F, Fut, T>(
        &self,
        task_type: TaskType,
        priority: TaskPriority,
        project_id: Option<i64>,
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
                workflow: None,
                cancel_token,
            },
            operation,
        )
        .await
    }

    async fn run_tracked_internal<F, Fut, T>(
        &self,
        input: RunTrackedTaskInput,
        operation: F,
    ) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<T, AppError>>,
    {
        let RunTrackedTaskInput {
            task_type,
            priority,
            project_id,
            cancel_token,
            context,
            workflow,
        } = input;
        let submitted_id = self
            .submit_internal(
                task_type,
                priority,
                project_id,
                context.as_ref(),
                workflow.as_ref(),
            )
            .await?;
        let submitted_guard =
            SubmittedTaskGuard::new(self.inner.clone(), self.db.clone(), submitted_id.clone());
        let Some(permit) = self
            .acquire_task_with_cancel(&submitted_id, cancel_token)
            .await?
        else {
            return self
                .cancel_submitted_task(&submitted_id, submitted_guard)
                .await;
        };
        self.finish_tracked_operation(permit, submitted_guard, operation().await)
            .await
    }
}
