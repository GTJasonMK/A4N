//! RAG 检索命令
//!
//! 对应 RagService。这里保留 Tauri 命令注册入口，具体实现按职责放在
//! `commands/rag/` 子模块中。

mod commands_ingest;
mod search;

pub use self::commands_ingest::*;
pub use self::search::*;
