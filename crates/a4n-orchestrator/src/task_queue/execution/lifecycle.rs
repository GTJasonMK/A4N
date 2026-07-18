use a4n_core::error::AppError;
use a4n_queue::priority::TaskPermit;
use tokio_util::sync::CancellationToken;

use super::super::{PersistentTaskQueue, SubmittedTaskGuard};

impl PersistentTaskQueue {
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

    pub fn submitted_task_guard(&self, task_id: String) -> SubmittedTaskGuard {
        SubmittedTaskGuard::new(self.inner.clone(), self.db.clone(), task_id)
    }
}
