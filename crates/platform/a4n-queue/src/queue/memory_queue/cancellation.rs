use a4n_error::AppError;

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

        if let Some(token) = self.cancel_tokens.get(task_id) {
            token.cancel();
        }
        let _ = waiting::remove_task(&self.waiting, task_id);
        self.update_status(task_id, TaskStatus::Cancelled, None);

        Ok(vec![task_id.to_string()])
    }

    /// 只通知正在执行的 operation；持久状态由上层唯一 lifecycle owner 更新。
    pub fn signal_task_cancel(&self, task_id: &str) -> Result<bool, AppError> {
        let task = self
            .tasks
            .get(task_id)
            .ok_or_else(|| AppError::NotFound(format!("任务不存在: {task_id}")))?;
        let active = matches!(task.status, TaskStatus::Pending | TaskStatus::Running);
        let pending = matches!(task.status, TaskStatus::Pending);
        drop(task);
        if !active {
            return Ok(false);
        }
        let token = self
            .cancel_tokens
            .get(task_id)
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("任务缺少取消令牌：{task_id}")))?;
        token.cancel();
        if pending {
            let _ = waiting::remove_task(&self.waiting, task_id);
        }
        Ok(true)
    }

    /// 把唯一持久 lifecycle owner 已提交的终态投影到内存队列。
    pub fn project_task_terminal(&self, task_id: &str, status: TaskStatus) -> Result<(), AppError> {
        if !super::is_terminal_status(&status) {
            return Err(AppError::InvalidArgument(
                "内存队列终态投影只接受 terminal status".to_string(),
            ));
        }
        if !self.tasks.contains_key(task_id) {
            return Err(AppError::NotFound(format!("任务不存在: {task_id}")));
        }
        if let Some(token) = self.cancel_tokens.get(task_id) {
            token.cancel();
        }
        let _ = waiting::remove_task(&self.waiting, task_id);
        self.update_status(task_id, status, None);
        self.cancel_tokens.remove(task_id);
        Ok(())
    }
}
