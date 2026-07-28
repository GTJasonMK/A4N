//! 漫画整页提示词优化 application。
//!
//! 统一拥有请求构造、workflow/checkpoint、LLM/结构化修复、提示词落库和下游清理编排。

mod checkpoint;
mod contracts;
mod execution;
mod request;

pub use contracts::{MangaPromptDownstreamCleanupPort, MangaPromptOptimizationInput};

#[cfg(test)]
mod tests;
