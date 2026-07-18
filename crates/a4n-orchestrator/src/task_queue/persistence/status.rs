use super::super::{PersistentTaskQueue, queue_db_error};
use a4n_core::error::AppError;

use self::workflow::sync_workflow_active_task;

#[cfg(test)]
mod tests;
mod workflow;

impl PersistentTaskQueue {
    pub(in crate::task_queue) async fn update_status(
        &self,
        task_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        a4n_db::queue::update_task_status(self.db.as_ref(), task_id, status, error)
            .await
            .map_err(queue_db_error)?;
        sync_workflow_active_task(self, task_id, status).await
    }

    pub async fn mark_done(&self, task_id: &str) -> Result<(), AppError> {
        self.update_status(task_id, "completed", None).await
    }

    pub async fn mark_failed(&self, task_id: &str, error: &str) -> Result<(), AppError> {
        self.update_status(task_id, "failed", Some(error)).await
    }
}
