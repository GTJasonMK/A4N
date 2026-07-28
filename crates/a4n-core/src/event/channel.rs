//! 任务通道管理
//!
//! 基于 task_id 命名空间的通道系统，支持任务注册、取消和进度跟踪。

mod contracts;
mod handle;
mod manager;

use contracts::MAX_TASK_PROGRESS_EVENTS;
pub use contracts::{TaskKind, TaskProgress, TaskProgressEvent, TaskSnapshot};
pub use handle::TaskHandle;
pub use manager::TaskManager;

#[cfg(test)]
mod tests;
