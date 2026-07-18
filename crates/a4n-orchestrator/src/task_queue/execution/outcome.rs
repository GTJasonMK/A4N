use a4n_core::error::AppError;
use a4n_queue::priority::TaskPermit;

use super::super::{PersistentTaskQueue, SubmittedTaskGuard};

impl PersistentTaskQueue {
    pub(super) async fn finish_tracked_operation<T>(
        &self,
        permit: TaskPermit,
        mut submitted_guard: SubmittedTaskGuard,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        let running_id = permit.task_id().to_string();
        match result {
            Ok(value) => {
                permit.complete();
                let status_result = self.mark_done(&running_id).await;
                submitted_guard.disarm();
                match status_result {
                    Ok(()) => Ok(value),
                    Err(status_error) => Err(AppError::Internal(anyhow::anyhow!(
                        "队列任务业务操作已完成，但同步完成状态失败: {status_error}"
                    ))),
                }
            }
            Err(err) => {
                let message = err.to_string();
                permit.fail(message.clone());
                let status_result = self.mark_failed(&running_id, &message).await;
                submitted_guard.disarm();
                match status_result {
                    Ok(()) => Err(err),
                    Err(status_error) => Err(AppError::Internal(anyhow::anyhow!(
                        "{err}；同步队列任务失败状态失败: {status_error}"
                    ))),
                }
            }
        }
    }
}
