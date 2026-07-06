//! AI 配置管理命令。
//!
//! 保持 `commands::config::*` 对外入口稳定，具体职责下沉到同名子模块。

mod embedding;
mod llm;
mod local_embedding;

pub use embedding::*;
pub use llm::*;
pub use local_embedding::*;
