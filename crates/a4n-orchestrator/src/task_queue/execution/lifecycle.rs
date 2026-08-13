use a4n_core::error::AppError;
use a4n_queue::priority::{TaskPermit, TaskStatus};
use tokio_util::sync::CancellationToken;

use super::super::{PersistentTaskQueue, SubmittedTaskGuard};

impl PersistentTaskQueue {
    /// Generation lifecycle persistence owns every database status write.
    /// The queue contributes only the in-memory execution permit.
    pub async fn acquire_generation_task(
        &self,
        task_id: &str,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.inner
            .acquire_task_with_cancel_non_reentrant(task_id, CancellationToken::new())
            .await
    }

    pub async fn acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        match self
            .inner
            .acquire_task_with_cancel(task_id, cancel_token)
            .await?
        {
            Some(permit) => Ok(Some(self.mark_permit_running(permit).await?)),
            None => Ok(None),
        }
    }

    pub(crate) fn signal_generation_cancel(&self, task_id: &str) -> Result<bool, AppError> {
        self.inner.signal_task_cancel(task_id)
    }

    pub(crate) fn project_generation_terminal(
        &self,
        task_id: &str,
        status: TaskStatus,
    ) -> Result<(), AppError> {
        self.inner.project_task_terminal(task_id, status)
    }

    pub fn submitted_task_guard(&self, task_id: String) -> SubmittedTaskGuard {
        SubmittedTaskGuard::new(self.inner.clone(), self.db.clone(), task_id)
    }
}
