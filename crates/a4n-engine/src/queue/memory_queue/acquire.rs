use std::sync::Arc;
use std::time::Duration;

use a4n_core::error::AppError;
use tokio::sync::Semaphore;
use tokio_util::sync::CancellationToken;

use super::kind::queue_kind_for_task_type;
use super::{MemoryTaskQueue, is_terminal_status, waiting};
use crate::queue::priority::{TaskPermit, TaskPriority, TaskProgress, TaskStatus, TaskType};

impl MemoryTaskQueue {
    fn make_permit(
        &self,
        task_id: String,
        task_type: TaskType,
        priority: TaskPriority,
        semaphore: Arc<Semaphore>,
    ) -> TaskPermit {
        let tasks = self.tasks.clone();
        let cancel_tokens = self.cancel_tokens.clone();
        let task_id_clone = task_id.clone();

        let on_complete = Box::new(move |status: TaskStatus| {
            if let Some(mut task) = tasks.get_mut(&task_id_clone)
                && !is_terminal_status(&task.status)
            {
                task.status = status;
                task.completed_at = Some(chrono::Utc::now().naive_utc());
            }

            cancel_tokens.remove(&task_id_clone);
            semaphore.add_permits(1);
        });

        TaskPermit::new(task_id, task_type, priority, on_complete)
    }

    pub async fn acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        loop {
            if cancel_token.is_cancelled() {
                return Ok(None);
            }

            let Some(task) = self.tasks.get(task_id).map(|task| task.clone()) else {
                return Err(AppError::Internal(anyhow::anyhow!("任务不存在：{task_id}")));
            };
            match task.status {
                TaskStatus::Pending => {}
                TaskStatus::Cancelled => return Ok(None),
                TaskStatus::Running => {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "任务已在运行，不能重复获取执行许可：{task_id}"
                    )));
                }
                TaskStatus::Completed
                | TaskStatus::Failed { .. }
                | TaskStatus::Interrupted { .. } => {
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "任务已结束，不能获取执行许可：{task_id}"
                    )));
                }
            }

            let kind = queue_kind_for_task_type(&task.task_type);
            let semaphore = self.semaphore_for_kind(kind).clone();
            match tokio::time::timeout(Duration::from_millis(500), semaphore.acquire()).await {
                Ok(Ok(permit)) => {
                    match waiting::pop_specific_if_next(&self.waiting, &self.tasks, task_id, kind) {
                        waiting::SpecificPop::Ready {
                            task_type,
                            priority,
                        } => {
                            self.update_status(
                                task_id,
                                TaskStatus::Running,
                                Some(TaskProgress {
                                    current: 0,
                                    total: 0,
                                    message: "开始执行".into(),
                                    meta: None,
                                    updated_at: Some(chrono::Utc::now().to_rfc3339()),
                                }),
                            );
                            std::mem::forget(permit);
                            return Ok(Some(self.make_permit(
                                task_id.to_string(),
                                task_type,
                                priority,
                                semaphore.clone(),
                            )));
                        }
                        waiting::SpecificPop::Blocked | waiting::SpecificPop::Missing => {
                            drop(permit);
                            tokio::time::sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
                Ok(Err(_)) => continue,
                Err(_) => {
                    if cancel_token.is_cancelled() {
                        return Ok(None);
                    }
                }
            }
        }
    }

    pub(super) async fn acquire_next_queued_task(
        &self,
        task_type: TaskType,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        let kind = queue_kind_for_task_type(&task_type);
        let semaphore = self.semaphore_for_kind(kind).clone();

        loop {
            if cancel_token.is_cancelled() {
                return Ok(None);
            }

            match tokio::time::timeout(Duration::from_millis(500), semaphore.acquire()).await {
                Ok(Ok(permit)) => {
                    let task = waiting::pop_matching_task(&self.waiting, &self.tasks, kind);

                    if let Some((task_id, queued_task_type, priority)) = task {
                        self.update_status(
                            &task_id,
                            TaskStatus::Running,
                            Some(TaskProgress {
                                current: 0,
                                total: 0,
                                message: "开始执行".into(),
                                meta: None,
                                updated_at: Some(chrono::Utc::now().to_rfc3339()),
                            }),
                        );

                        std::mem::forget(permit);

                        return Ok(Some(self.make_permit(
                            task_id,
                            queued_task_type,
                            priority,
                            semaphore.clone(),
                        )));
                    }

                    drop(permit);
                    tokio::time::sleep(Duration::from_millis(100)).await;
                }
                Ok(Err(_)) => continue,
                Err(_) => {
                    if cancel_token.is_cancelled() {
                        return Ok(None);
                    }
                }
            }
        }
    }
}
