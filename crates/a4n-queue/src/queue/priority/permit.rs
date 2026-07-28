use super::{TaskKind, TaskPriority, TaskStatus};
use tokio_util::sync::CancellationToken;

/// 任务许可（RAII 自动释放）
///
/// 获取许可后必须调用 `complete()` 或 `fail()` 释放槽位。
pub struct TaskPermit {
    task_id: String,
    task_type: TaskKind,
    priority: TaskPriority,
    cancellation_token: CancellationToken,
    on_complete: Option<Box<dyn FnOnce(TaskStatus) + Send>>,
}

impl TaskPermit {
    pub fn new(
        task_id: String,
        task_type: TaskKind,
        priority: TaskPriority,
        on_complete: Box<dyn FnOnce(TaskStatus) + Send>,
    ) -> Self {
        Self::with_cancellation_token(
            task_id,
            task_type,
            priority,
            CancellationToken::new(),
            on_complete,
        )
    }

    pub(crate) fn with_cancellation_token(
        task_id: String,
        task_type: TaskKind,
        priority: TaskPriority,
        cancellation_token: CancellationToken,
        on_complete: Box<dyn FnOnce(TaskStatus) + Send>,
    ) -> Self {
        Self {
            task_id,
            task_type,
            priority,
            cancellation_token,
            on_complete: Some(on_complete),
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn task_type(&self) -> &TaskKind {
        &self.task_type
    }

    pub fn priority(&self) -> TaskPriority {
        self.priority
    }

    pub fn cancellation_token(&self) -> CancellationToken {
        self.cancellation_token.clone()
    }

    /// 标记任务完成（释放槽位）
    pub fn complete(mut self) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Completed);
        }
    }

    /// 标记任务失败（释放槽位）
    pub fn fail(mut self, error: String) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Failed { error });
        }
    }

    pub fn cancel(mut self) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Cancelled);
        }
    }

    pub fn interrupt(mut self, reason: Option<String>) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Interrupted { reason });
        }
    }
}

impl Drop for TaskPermit {
    fn drop(&mut self) {
        if let Some(on_complete) = self.on_complete.take() {
            on_complete(TaskStatus::Failed {
                error: "任务许可被释放但未显式完成".to_string(),
            });
        }
    }
}

impl std::fmt::Debug for TaskPermit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TaskPermit")
            .field("task_id", &self.task_id)
            .field("task_type", &self.task_type)
            .field("priority", &self.priority)
            .finish()
    }
}
