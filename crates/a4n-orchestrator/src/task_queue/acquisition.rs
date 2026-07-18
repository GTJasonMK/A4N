use a4n_core::error::AppError;
use a4n_queue::priority::TaskPermit;

use super::PersistentTaskQueue;

impl PersistentTaskQueue {
    pub(super) async fn mark_permit_running(
        &self,
        permit: TaskPermit,
    ) -> Result<TaskPermit, AppError> {
        let task_id = permit.task_id().to_string();
        self.update_status(&task_id, "running", None).await?;
        Ok(permit)
    }
}
