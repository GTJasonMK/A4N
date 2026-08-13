use super::super::{PersistentTaskQueue, queue_db_error};
use a4n_core::error::AppError;

#[cfg(test)]
mod tests;

impl PersistentTaskQueue {
    pub(in crate::task_queue) async fn update_status(
        &self,
        task_id: &str,
        status: &str,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        a4n_db::queue::update_task_status(self.db.as_ref(), task_id, status, error)
            .await
            .map_err(queue_db_error)
    }
}
