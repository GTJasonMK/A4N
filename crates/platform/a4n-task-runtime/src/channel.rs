//! 任务通道管理
//!
//! 基于 task_id 命名空间的通道系统，支持任务注册、取消和进度跟踪。

mod contracts;
mod handle;
mod manager;

pub use a4n_domain::task_queue::TaskKind;
use a4n_error::AppError;
pub use contracts::{TaskProgress, TaskProgressEvent, TaskSnapshot};
pub use handle::TaskHandle;
pub use manager::TaskManager;

pub fn normalize_task_id(task_id: &str) -> Result<String, AppError> {
    let task_id = task_id.trim();
    if task_id.is_empty() {
        return Err(AppError::InvalidArgument("task_id 不能为空".to_string()));
    }
    if task_id.len() > 160 {
        return Err(AppError::InvalidArgument(
            "task_id 长度不能超过 160 个字符".to_string(),
        ));
    }
    if !task_id
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b':' | b'.'))
    {
        return Err(AppError::InvalidArgument(
            "task_id 只能包含 ASCII 字母、数字、-、_、:、.".to_string(),
        ));
    }
    Ok(task_id.to_string())
}

#[cfg(test)]
mod tests;
