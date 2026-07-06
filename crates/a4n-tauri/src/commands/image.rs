//! 图片管理命令。
//!
//! 保持 `commands::image::*` 的 Tauri 命令入口稳定，图片记录、配置和生成实现按职责下沉。

mod configs;
mod generation;
mod records;

pub use self::configs::*;
pub use self::generation::*;
pub use self::records::*;
