//! Tera 提示词引擎。
//!
//! 从 Markdown 目录加载默认提示词，解析模板本地字段和中央 registry 元数据，
//! 并支持项目级覆盖、版本、Git 持久化与变量校验。

mod git_persistence;
mod loader;
mod mutation;
mod operations;
mod overrides;
mod state;
mod variables;
mod version;

#[cfg(test)]
mod tests;

pub use state::TeraPromptEngine;
