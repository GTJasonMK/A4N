//! TaskQueue trait：优先级任务调度
//!
//! 支持高/中/低优先级调度和并发控制（信号量）。

mod kinds;
mod models;
mod permit;
mod queue;
mod status;

pub use kinds::*;
pub use models::*;
pub use permit::*;
pub use queue::*;
pub use status::*;

#[cfg(test)]
mod tests;
