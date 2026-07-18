//! 基于 git2 的提示词版本仓库
//!
//! 每个提示词作为仓库内的一个文件（`{name}.md`），每次写入触发一次提交。
//! 历史检索、按 commit sha 取旧版本都通过 git2 完成。
//!
//! 设计要点：
//! - 仓库通过 `open_or_init` 在 `data_dir/prompts.git` 自动建库
//! - 所有 git2 API 是同步阻塞的，async 调用方需自行用 `spawn_blocking` 包装
//! - 提交作者从环境/参数读取，避免依赖外部 git config

mod commit;
mod files;
mod history;
mod naming;
mod store;
mod trait_impl;

pub use store::GitPromptStore;

#[cfg(test)]
mod tests;
