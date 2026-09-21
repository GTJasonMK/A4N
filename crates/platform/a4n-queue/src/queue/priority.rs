//! 优先级任务调度合同
//!
//! 支持高/中/低优先级调度和并发控制（信号量）。

mod kinds;
mod models;
mod permit;
mod status;

pub use kinds::{TaskKind, TaskPriority};
pub use models::{QueueSlotStats, QueueStats, TaskInfo, TaskProgress};
pub use permit::TaskPermit;
pub use status::TaskStatus;

#[cfg(test)]
mod tests;
