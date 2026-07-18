use serde::{Deserialize, Serialize};

use super::defaults::{
    default_llm_tokens_blueprint, default_llm_tokens_chapter_analysis,
    default_llm_tokens_chapter_generation, default_llm_tokens_character_discovery,
    default_llm_tokens_character_sync, default_llm_tokens_connection_test,
    default_llm_tokens_extra_long, default_llm_tokens_json_standard, default_llm_tokens_long,
    default_llm_tokens_media_prompt, default_llm_tokens_outline, default_llm_tokens_short,
    default_llm_tokens_standard,
};

/// LLM 各场景请求输出预算配置。
///
/// 运行时优先从 system_settings.key = "llm_max_tokens" 读取；这里的默认值同时作为
/// 配置文件默认值和数据库记录缺失时的默认预算。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LlmMaxTokensConfig {
    #[serde(default = "default_llm_tokens_connection_test")]
    pub connection_test: usize,
    #[serde(default = "default_llm_tokens_short")]
    pub short: usize,
    #[serde(default = "default_llm_tokens_standard")]
    pub standard: usize,
    #[serde(default = "default_llm_tokens_json_standard")]
    pub json_standard: usize,
    #[serde(default = "default_llm_tokens_long")]
    pub long: usize,
    #[serde(default = "default_llm_tokens_extra_long")]
    pub extra_long: usize,
    #[serde(default = "default_llm_tokens_chapter_generation")]
    pub chapter_generation: usize,
    #[serde(default = "default_llm_tokens_chapter_analysis")]
    pub chapter_analysis: usize,
    #[serde(default = "default_llm_tokens_outline")]
    pub outline: usize,
    #[serde(default = "default_llm_tokens_blueprint")]
    pub blueprint: usize,
    #[serde(default = "default_llm_tokens_character_discovery")]
    pub character_discovery: usize,
    #[serde(default = "default_llm_tokens_character_sync")]
    pub character_sync: usize,
    #[serde(default = "default_llm_tokens_media_prompt")]
    pub media_prompt: usize,
}
