//! 大纲系统命令。
//!
//! Tauri 层只负责命令签名和窗口进度；大纲业务逻辑统一在
//! `a4n_runtime::host_api::outline_records`。

mod records;
mod task;

pub use records::*;
pub use task::*;
