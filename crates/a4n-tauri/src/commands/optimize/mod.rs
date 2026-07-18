//! 内容优化命令。
//!
//! Tauri 层只保留命令入口和桌面任务事件；优化会话、分析、补丁和对话流程
//! 统一由 `a4n-runtime::actions::optimize` 承载。

mod adapters;
mod analysis;
mod chat;
mod patches;
mod records;
mod task;

pub use analysis::*;
pub use chat::*;
pub use patches::*;
pub use records::*;
