use std::time::Duration;

use a4n_domain::task_queue::LlmConcurrencyLimit;
use a4n_error::AppError;
use tokio_util::sync::CancellationToken;

use super::kind::{QueueKind, queue_kind_for_task_type};
use super::{MemoryTaskQueue, is_terminal_status, waiting};
use crate::queue::priority::{TaskKind, TaskPermit, TaskPriority, TaskProgress, TaskStatus};

impl MemoryTaskQueue {
    fn make_permit(
        &self,
        task_id: String,
        task_type: TaskKind,
        priority: TaskPriority,
        cancellation_token: CancellationToken,
        slot: Option<Box<dyn Send>>,
    ) -> TaskPermit {
        let queue = self.clone();
        let task_id_clone = task_id.clone();

        let on_complete = Box::new(move |status: TaskStatus| {
            if let Some(mut task) = queue.tasks.get_mut(&task_id_clone)
                && !is_terminal_status(&task.status)
            {
                task.status = status;
                task.completed_at = Some(chrono::Utc::now().naive_utc());
                drop(task);
                queue.retain_terminal_task(&task_id_clone);
            }

            queue.cancel_tokens.remove(&task_id_clone);
            drop(slot);
        });

        TaskPermit::with_cancellation_token(
            task_id,
            task_type,
            priority,
            cancellation_token,
            on_complete,
        )
    }

    pub async fn acquire_task_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.acquire_task_with_cancel_inner(task_id, cancel_token, true, None)
            .await
    }

    pub async fn acquire_task_with_cancel_non_reentrant(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.acquire_task_with_cancel_inner(task_id, cancel_token, false, None)
            .await
    }

    pub async fn acquire_llm_call_with_cancel(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
        limit: LlmConcurrencyLimit,
    ) -> Result<Option<TaskPermit>, AppError> {
        self.acquire_task_with_cancel_inner(task_id, cancel_token, true, Some(limit))
            .await
    }

    async fn acquire_task_with_cancel_inner(
        &self,
        task_id: &str,
        cancel_token: CancellationToken,
        allow_reentrant: bool,
        llm_limit: Option<LlmConcurrencyLimit>,
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
            let task_cancellation = self
                .cancel_tokens
                .get(task_id)
                .map(|token| token.clone())
                .ok_or_else(|| {
                    AppError::Internal(anyhow::anyhow!("任务缺少取消令牌：{task_id}"))
                })?;
            if task_cancellation.is_cancelled() {
                return Ok(None);
            }

            let kind = queue_kind_for_task_type(&task.task_type);
            let semaphore = self.semaphore_for_kind(kind).clone();
            let reentrant = allow_reentrant
                && ((matches!(kind, QueueKind::Llm) && crate::scope::llm_slot_is_held())
                    || (matches!(kind, QueueKind::Image) && crate::scope::image_slot_is_held())
                    || (matches!(kind, QueueKind::Video) && crate::scope::video_slot_is_held()));
            // Keep process admission before provider admission. Taking these in
            // the opposite order could deadlock a workflow that already owns a slot.
            let mut slot: Option<Box<dyn Send>> = if reentrant {
                if !waiting::remove_task(&self.waiting, task_id) {
                    if task_cancellation.is_cancelled() || cancel_token.is_cancelled() {
                        return Ok(None);
                    }
                    return Err(AppError::Internal(anyhow::anyhow!(
                        "任务不在等待队列中：{task_id}"
                    )));
                }
                None
            } else {
                let acquired = tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => return Ok(None),
                    _ = task_cancellation.cancelled() => return Ok(None),
                    result = tokio::time::timeout(Duration::from_millis(500), semaphore.clone().acquire_owned()) => result,
                };
                let permit = match acquired {
                    Ok(Ok(permit)) => permit,
                    Ok(Err(_)) | Err(_) => continue,
                };
                match waiting::pop_specific_if_next(&self.waiting, &self.tasks, task_id, kind) {
                    waiting::SpecificPop::Ready => Some(Box::new(permit)),
                    waiting::SpecificPop::Blocked | waiting::SpecificPop::Missing => {
                        drop(permit);
                        tokio::time::sleep(Duration::from_millis(100)).await;
                        continue;
                    }
                }
            };
            if let Some(limit) = llm_limit.as_ref() {
                if task.task_type != TaskKind::LlmCall {
                    return Err(AppError::InvalidArgument(
                        "LLM 并发限制只能用于 llm_call".into(),
                    ));
                }
                let provider_slot = tokio::select! {
                    biased;
                    _ = cancel_token.cancelled() => return Ok(None),
                    _ = task_cancellation.cancelled() => return Ok(None),
                    permit = self.llm_limits.acquire(limit, task.priority) => permit?,
                };
                slot = Some(Box::new((slot, provider_slot)));
            }
            if task_cancellation.is_cancelled() || cancel_token.is_cancelled() {
                return Ok(None);
            }
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
            return Ok(Some(self.make_permit(
                task_id.to_string(),
                task.task_type,
                task.priority,
                task_cancellation,
                slot,
            )));
        }
    }
}
