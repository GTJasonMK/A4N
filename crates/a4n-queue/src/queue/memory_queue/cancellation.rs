use a4n_core::error::AppError;

use super::{MemoryTaskQueue, waiting};
use crate::queue::priority::TaskStatus;

impl MemoryTaskQueue {
    pub fn cancel_task_now(&self, task_id: &str) -> Result<Vec<String>, AppError> {
        let task = self
            .tasks
            .get(task_id)
            .ok_or_else(|| AppError::NotFound(format!("任务不存在: {task_id}")))?;
        let should_cancel = matches!(task.status, TaskStatus::Pending | TaskStatus::Running);
        drop(task);

        if !should_cancel {
            return Ok(Vec::new());
        }

        if let Some((_, token)) = self.cancel_tokens.remove(task_id) {
            token.cancel();
        }
        waiting::remove_task(&self.waiting, task_id);
        self.update_status(task_id, TaskStatus::Cancelled, None);

        Ok(vec![task_id.to_string()])
    }
}
