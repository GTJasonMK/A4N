use super::{TaskPriority, TaskStatus, TaskType};

/// 任务许可（RAII 自动释放）
///
/// 获取许可后必须调用 `complete()` 或 `fail()` 释放槽位。
pub struct TaskPermit {
    task_id: String,
    task_type: TaskType,
    priority: TaskPriority,
    on_complete: Option<Box<dyn FnOnce(TaskStatus) + Send>>,
}

impl TaskPermit {
    pub fn new(
        task_id: String,
        task_type: TaskType,
        priority: TaskPriority,
        on_complete: Box<dyn FnOnce(TaskStatus) + Send>,
    ) -> Self {
        Self {
            task_id,
            task_type,
            priority,
            on_complete: Some(on_complete),
        }
    }

    pub fn task_id(&self) -> &str {
        &self.task_id
    }

    pub fn task_type(&self) -> &TaskType {
        &self.task_type
    }

    pub fn priority(&self) -> TaskPriority {
        self.priority
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
