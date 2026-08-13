use std::future::Future;

use a4n_core::error::AppError;
use a4n_queue::priority::{TaskKind, TaskPriority};
use tokio_util::sync::CancellationToken;

use super::{PersistentTaskQueue, RunTrackedTaskInput};

mod cancellation;
mod contexts;
mod entrypoints;
mod lifecycle;

impl PersistentTaskQueue {
    pub async fn run_tracked_with_cancel<F, Fut, T>(
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
        let running_task = if let Some(context) = context {
            let task_id = self
                .submit_internal(
                    task_type,
                    priority,
                    project_id,
                    Some(&context),
                    workflow.as_ref(),
                )
                .await?;
            let mut guard = self.submitted_task_guard(task_id.clone());
            let Some(permit) = self
                .acquire_task_with_cancel(&task_id, cancel_token)
                .await?
            else {
                return self.cancel_submitted_task(&task_id, guard).await;
            };
            guard.disarm();
            crate::task_queue::RunningTask::new(self.db.clone(), permit)
        } else {
            self.start_task_with_cancel(task_type, priority, project_id, workflow, cancel_token)
                .await?
        };
        let result = match task_type {
            TaskKind::LlmCall | TaskKind::TextGeneration => {
                a4n_queue::scope::with_llm_slot(operation()).await
            }
            _ => operation().await,
        };
        running_task.finish(result).await
    }
}
