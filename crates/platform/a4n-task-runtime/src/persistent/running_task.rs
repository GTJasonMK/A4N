use a4n_db::DbHandle;
use a4n_error::AppError;
use a4n_queue::priority::TaskPermit;

use super::{queue_db_error, submitted_guard::persist_dropped_status};

pub struct RunningTask {
    db: DbHandle,
    permit: Option<TaskPermit>,
}

impl RunningTask {
    pub fn cancellation_token(&self) -> tokio_util::sync::CancellationToken {
        self.permit
            .as_ref()
            .expect("running task owns its permit")
            .cancellation_token()
    }

    pub(in crate::persistent) fn new(db: DbHandle, permit: TaskPermit) -> Self {
        Self {
            db,
            permit: Some(permit),
        }
    }

    pub async fn complete(mut self) -> Result<(), AppError> {
        let permit = self.take_permit();
        let task_id = permit.task_id().to_string();
        permit.complete();
        a4n_db::queue::update_task_status(&self.db, &task_id, "completed", None)
            .await
            .map_err(queue_db_error)
    }

    pub async fn fail(mut self, error: &str) -> Result<(), AppError> {
        let permit = self.take_permit();
        let task_id = permit.task_id().to_string();
        permit.fail(error.to_string());
        a4n_db::queue::update_task_status(&self.db, &task_id, "failed", Some(error))
            .await
            .map_err(queue_db_error)
    }

    pub async fn cancel(mut self) -> Result<(), AppError> {
        let permit = self.take_permit();
        let task_id = permit.task_id().to_string();
        permit.cancel();
        a4n_db::queue::update_task_status(&self.db, &task_id, "cancelled", None)
            .await
            .map_err(queue_db_error)
    }

    pub(in crate::persistent) async fn finish<T>(
        self,
        result: Result<T, AppError>,
    ) -> Result<T, AppError> {
        match result {
            Ok(value) => self
                .complete()
                .await
                .map(|()| value)
                .map_err(|status_error| {
                    AppError::Internal(anyhow::anyhow!(
                        "队列任务业务操作已完成，但同步完成状态失败: {status_error}"
                    ))
                }),
            Err(error) => {
                let message = error.to_string();
                match self.fail(&message).await {
                    Ok(()) => Err(error),
                    Err(status_error) => Err(AppError::Internal(anyhow::anyhow!(
                        "{error}；同步队列任务失败状态失败: {status_error}"
                    ))),
                }
            }
        }
    }

    fn take_permit(&mut self) -> TaskPermit {
        self.permit
            .take()
            .expect("running task permit can only be settled once")
    }
}

impl Drop for RunningTask {
    fn drop(&mut self) {
        let Some(permit) = self.permit.take() else {
            return;
        };
        let task_id = permit.task_id().to_string();
        let message = "运行中的队列任务在完成前被释放";
        permit.fail(message.to_string());
        persist_dropped_status(self.db.clone(), task_id, "failed", Some(message));
    }
}
