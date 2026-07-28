use a4n_core::error::AppError;
use a4n_queue::priority::{TaskPermit, TaskStatus};
use tokio_util::sync::CancellationToken;

use super::super::{PersistentTaskQueue, SubmittedTaskGuard};

impl PersistentTaskQueue {
    /// Generation tasks already have their authoritative lifecycle in the
    /// generation checkpoint. Acquire the memory permit without the generic
    /// `mark_permit_running` write, then mirror the queue row exactly once.
    pub async fn acquire_generation_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        let Some(permit) = self
            .inner
            .acquire_task_with_cancel_non_reentrant(task_id, cancel_token)
            .await?
        else {
            return Ok(None);
        };
        if let Err(error) = self.update_status(task_id, "running", None).await {
            permit.fail(error.to_string());
            return Err(error);
        }
        Ok(Some(permit))
    }

    pub async fn complete_generation_task(&self, permit: TaskPermit) -> Result<(), AppError> {
        let task_id = permit.task_id().to_string();
        match self.update_status(&task_id, "completed", None).await {
            Ok(()) => {
                permit.complete();
                Ok(())
            }
            Err(error) => {
                let message = error.to_string();
                permit.fail(message);
                Err(error)
            }
        }
    }

    pub async fn fail_generation_task(
        &self,
        permit: TaskPermit,
        error: &str,
    ) -> Result<(), AppError> {
        let task_id = permit.task_id().to_string();
        match self.update_status(&task_id, "failed", Some(error)).await {
            Ok(()) => {
                permit.fail(error.to_string());
                Ok(())
            }
            Err(status_error) => {
                permit.fail(status_error.to_string());
                Err(status_error)
            }
        }
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
