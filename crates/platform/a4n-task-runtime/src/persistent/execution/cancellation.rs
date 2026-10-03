use a4n_error::AppError;

use super::super::{PersistentTaskQueue, SubmittedTaskGuard};

impl PersistentTaskQueue {
    pub(super) async fn cancel_submitted_task<T>(
        &self,
        submitted_id: &str,
        mut submitted_guard: SubmittedTaskGuard,
    ) -> Result<T, AppError> {
        let cancel_result = self.cancel(submitted_id).await;
        submitted_guard.disarm();
        match cancel_result {
            Ok(true) => Err(AppError::Cancelled),
            Ok(false) => Err(AppError::InvalidState {
                current: "已终态".to_string(),
                operation: "取消未获得执行权的队列任务".to_string(),
            }),
            Err(status_error) => Err(AppError::Internal(anyhow::anyhow!(
                "任务已取消；同步队列任务取消状态失败: {status_error}"
            ))),
        }
    }
}
