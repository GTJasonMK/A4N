//! 持久化任务队列
//!
//! 基于 MemoryTaskQueue 的数据库持久化包装器。
//! 任务状态变更时自动同步到数据库，启动时恢复未完成任务。

mod access;
mod acquisition;
mod concurrency;
mod contracts;
mod errors;
mod execution;
mod persistence;
mod running_task;
mod submitted_guard;
#[cfg(test)]
mod test_support;

pub use contracts::PersistentTaskQueue;
pub(in crate::task_queue) use contracts::RunTrackedTaskInput;
pub(in crate::task_queue) use errors::queue_db_error;
pub use running_task::RunningTask;
pub(in crate::task_queue) use submitted_guard::SubmittedTaskGuard;
