//! 漫画整页提示词优化 application。

mod contracts;
mod execution;
mod request;

pub use contracts::{MangaPromptDownstreamCleanupPort, MangaPromptOptimizationInput};

#[cfg(test)]
mod tests;
