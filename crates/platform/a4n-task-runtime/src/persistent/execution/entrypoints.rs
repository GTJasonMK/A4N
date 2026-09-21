use std::future::Future;

use a4n_error::AppError;
use a4n_queue::{
    TaskFamilyContext,
    priority::{TaskKind, TaskPriority},
};
use tokio_util::sync::CancellationToken;

use super::super::PersistentTaskQueue;

impl PersistentTaskQueue {
    pub async fn run_tracked<F, Fut, T>(
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
        task_type: TaskKind,
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
}
