//! 应用全局配置
//!
//! 定义应用的完整配置结构，支持从文件加载和环境变量覆盖。

mod contracts;
mod defaults;
mod portable;

pub use a4n_domain::{
    DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, LlmMaxTokensConfig,
    llm_token_profiles,
};
pub use contracts::{
    AppConfig, EmbeddingConfig, ImageGenConfig, LlmConfig, SecurityConfig, SystemConfig,
};
use defaults::*;
pub use portable::{portable_home_dir, resolve_path_in_portable_home, resolve_portable_path};

#[cfg(test)]
mod tests;
