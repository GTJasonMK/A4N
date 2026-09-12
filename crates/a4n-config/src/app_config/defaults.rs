use crate::optional_non_empty_env_path;
use a4n_domain::{DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS};
use std::path::PathBuf;

use super::contracts::{
    AppConfig, EmbeddingConfig, ImageGenConfig, LlmConfig, SecurityConfig, SystemConfig,
};
use super::portable::{portable_home_dir, resolve_portable_path};

pub(super) fn default_data_dir() -> PathBuf {
    if let Some(path) = optional_non_empty_env_path("A4N_DATA_DIR").expect("A4N_DATA_DIR 配置无效")
    {
        return resolve_portable_path(path);
    }
    portable_home_dir().join("data")
}
pub(super) fn default_cache_dir() -> PathBuf {
    default_data_dir().join("cache")
}
pub(super) fn default_temp_dir() -> PathBuf {
    default_data_dir().join("tmp")
}
pub(super) fn default_log_dir() -> PathBuf {
    default_data_dir().join("logs")
}
pub(super) fn default_database_url() -> String {
    String::new()
}
pub(super) fn default_llm_provider() -> String {
    "deepseek".into()
}
pub(super) fn default_llm_base_url() -> String {
    "https://api.deepseek.com/v1".into()
}
pub(super) fn default_llm_model() -> String {
    "deepseek-v4-flash".into()
}
pub(super) fn default_llm_max_concurrent() -> usize {
    3
}
pub(super) fn default_llm_timeout_secs() -> u64 {
    300
}
pub(super) fn default_llm_max_context_tokens() -> usize {
    DEFAULT_LLM_MAX_CONTEXT_TOKENS
}
pub(super) fn default_llm_max_output_tokens() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}
pub(super) fn default_embedding_backend() -> String {
    "local_hash".into()
}
pub(super) fn default_embedding_base_url() -> String {
    String::new()
}
pub(super) fn default_embedding_model() -> String {
    "local-sha256-hash".into()
}
pub(super) fn default_embedding_dim() -> usize {
    384
}
pub(super) fn default_embedding_batch_size() -> usize {
    100
}
pub(super) fn default_image_backend() -> String {
    "openai-compatible-image".into()
}
pub(super) fn default_image_model() -> String {
    "dall-e-3".into()
}
pub(super) fn default_image_max_concurrent() -> usize {
    2
}
pub(super) fn default_event_bus_capacity() -> usize {
    1024
}

impl Default for AppConfig {
    fn default() -> Self {
        let data_dir = default_data_dir();
        Self {
            database_url: default_database_url(),
            cache_dir: data_dir.join("cache"),
            temp_dir: data_dir.join("tmp"),
            log_dir: data_dir.join("logs"),
            data_dir,
            llm: LlmConfig::default(),
            embedding: EmbeddingConfig::default(),
            image_gen: ImageGenConfig::default(),
            security: SecurityConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            primary_provider: default_llm_provider(),
            base_url: default_llm_base_url(),
            api_key: None,
            model: default_llm_model(),
            max_concurrent: default_llm_max_concurrent(),
            timeout_secs: default_llm_timeout_secs(),
            max_context_tokens: default_llm_max_context_tokens(),
            max_output_tokens: default_llm_max_output_tokens(),
            extra_body_json: None,
        }
    }
}

impl Default for EmbeddingConfig {
    fn default() -> Self {
        Self {
            backend: default_embedding_backend(),
            base_url: default_embedding_base_url(),
            api_key: None,
            model: default_embedding_model(),
            dimension: default_embedding_dim(),
            batch_size: default_embedding_batch_size(),
            local_model_path: None,
        }
    }
}

impl Default for ImageGenConfig {
    fn default() -> Self {
        Self {
            backend: default_image_backend(),
            base_url: None,
            api_key: None,
            model: default_image_model(),
            max_concurrent: default_image_max_concurrent(),
            supported_sizes: a4n_domain::default_image_sizes(),
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            event_bus_capacity: default_event_bus_capacity(),
        }
    }
}
