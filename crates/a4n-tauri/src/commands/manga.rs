//! 漫画生成命令。
//!
//! 保持 `commands::manga::*` 的 Tauri 命令入口稳定，分镜、状态和流水线实现按职责下沉。

mod panels;
mod pipeline;
mod status;

pub use self::panels::*;
pub use self::pipeline::*;
pub use self::status::*;
