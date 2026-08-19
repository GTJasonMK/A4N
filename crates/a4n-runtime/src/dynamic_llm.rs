//! 动态 LLM Provider：每次调用时从数据库解析当前激活的 LLM 配置。

mod budget;
mod config;
mod context;
mod contracts;
mod llm_provider;
mod provider;
mod streams;
mod trace;

pub use budget::resolve_effective_llm_preview_budget_for_config;
pub use contracts::{DynamicLlmProvider, EffectiveLlmPreviewBudget, LLM_MAX_TOKENS_SETTING_KEY};

#[cfg(test)]
mod tests;
