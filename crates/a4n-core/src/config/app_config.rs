//! 应用全局配置
//!
//! 定义应用的完整配置结构，支持从文件加载和环境变量覆盖。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

mod portable;
mod tokens;

use portable::non_empty_env_path;
pub use portable::{portable_home_dir, resolve_path_in_portable_home, resolve_portable_path};
pub use tokens::{LlmMaxTokensConfig, llm_token_profiles};

pub const DEFAULT_LLM_MAX_CONTEXT_TOKENS: usize = 128_000;
pub const DEFAULT_LLM_MAX_OUTPUT_TOKENS: usize = 8_192;

/// 应用全局配置
///
/// 包含数据库、存储路径、LLM、嵌入、图片生成和系统运行参数。
/// 支持 TOML/YAML 文件加载和环境变量覆盖。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// PostgreSQL 数据库连接 URL。
    #[serde(default = "default_database_url")]
    pub database_url: String,

    /// 数据存储根目录
    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    /// 缓存目录
    #[serde(default = "default_cache_dir")]
    pub cache_dir: PathBuf,

    /// 临时文件目录
    #[serde(default = "default_temp_dir")]
    pub temp_dir: PathBuf,

    /// 日志目录
    #[serde(default = "default_log_dir")]
    pub log_dir: PathBuf,

    /// LLM 配置
    #[serde(default)]
    pub llm: LlmConfig,

    /// 嵌入模型配置
    #[serde(default)]
    pub embedding: EmbeddingConfig,

    /// 图片生成配置
    #[serde(default)]
    pub image_gen: ImageGenConfig,

    /// 安全配置
    #[serde(default)]
    pub security: SecurityConfig,

    /// 系统设置
    #[serde(default)]
    pub system: SystemConfig,
}

/// LLM 服务配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// 主 Provider 类型
    #[serde(default = "default_llm_provider")]
    pub primary_provider: String,

    /// API 基础 URL
    #[serde(default = "default_llm_base_url")]
    pub base_url: String,

    /// API Key（支持环境变量 ${OPENAI_API_KEY}）
    pub api_key: Option<String>,

    /// 默认模型名称
    #[serde(default = "default_llm_model")]
    pub model: String,

    /// 最大并发请求数
    #[serde(default = "default_llm_max_concurrent")]
    pub max_concurrent: usize,

    /// 请求超时（秒）
    #[serde(default = "default_llm_timeout_secs")]
    pub timeout_secs: u64,

    /// 模型总上下文窗口（输入 + 输出）。
    #[serde(default = "default_llm_max_context_tokens")]
    pub max_context_tokens: usize,

    /// 模型单次最大输出 token 数。
    #[serde(default = "default_llm_max_output_tokens")]
    pub max_output_tokens: usize,

    /// Provider-specific extra JSON fields merged into chat/completions requests.
    #[serde(default)]
    pub request_options_json: Option<Value>,

    /// 重试次数
    #[serde(default = "default_retry_times")]
    pub retry_times: u32,

    /// LLM 各场景请求输出预算配置
    #[serde(default)]
    pub max_tokens: LlmMaxTokensConfig,
}

/// 嵌入模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingConfig {
    /// 嵌入后端类型：api / fastembed / local_hash
    #[serde(default = "default_embedding_backend")]
    pub backend: String,

    /// API 模式下的 base URL
    #[serde(default = "default_embedding_base_url")]
    pub base_url: String,

    /// API Key
    pub api_key: Option<String>,

    /// 模型名称
    #[serde(default = "default_embedding_model")]
    pub model: String,

    /// 向量维度
    #[serde(default = "default_embedding_dim")]
    pub dimension: usize,

    /// 批量大小
    #[serde(default = "default_embedding_batch_size")]
    pub batch_size: usize,

    /// fastembed 本地模型缓存路径
    pub local_model_path: Option<PathBuf>,
}

/// 图片生成配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageGenConfig {
    /// 后端类型
    #[serde(default = "default_image_backend")]
    pub backend: String,

    /// API 模式下的 base URL
    pub base_url: Option<String>,

    /// API Key
    pub api_key: Option<String>,

    /// 默认模型
    #[serde(default = "default_image_model")]
    pub model: String,

    /// 最大并发数
    #[serde(default = "default_image_max_concurrent")]
    pub max_concurrent: usize,

    /// 默认图片宽度
    #[serde(default = "default_image_width")]
    pub default_width: u32,

    /// 默认图片高度
    #[serde(default = "default_image_height")]
    pub default_height: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecurityConfig {
    pub jwt_secret: Option<String>,
}

/// 系统级配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    /// 任务队列容量
    #[serde(default = "default_queue_capacity")]
    pub queue_capacity: usize,

    /// 事件总线缓冲区大小
    #[serde(default = "default_event_bus_capacity")]
    pub event_bus_capacity: usize,
}

// ========== 默认值函数 ==========

fn default_data_dir() -> PathBuf {
    if let Some(path) = non_empty_env_path("A4N_DATA_DIR") {
        return resolve_portable_path(path);
    }
    portable_home_dir().join("data")
}
fn default_cache_dir() -> PathBuf {
    default_data_dir().join("cache")
}
fn default_temp_dir() -> PathBuf {
    default_data_dir().join("tmp")
}
fn default_log_dir() -> PathBuf {
    default_data_dir().join("logs")
}
fn default_database_url() -> String {
    String::new()
}
fn default_llm_provider() -> String {
    "deepseek".into()
}
fn default_llm_base_url() -> String {
    "https://api.deepseek.com/v1".into()
}
fn default_llm_model() -> String {
    "deepseek-v4-flash".into()
}
fn default_llm_max_concurrent() -> usize {
    3
}
fn default_llm_timeout_secs() -> u64 {
    300
}
fn default_llm_max_context_tokens() -> usize {
    DEFAULT_LLM_MAX_CONTEXT_TOKENS
}
fn default_llm_max_output_tokens() -> usize {
    DEFAULT_LLM_MAX_OUTPUT_TOKENS
}
fn default_retry_times() -> u32 {
    2
}
fn default_embedding_backend() -> String {
    "local_hash".into()
}
fn default_embedding_base_url() -> String {
    String::new()
}
fn default_embedding_model() -> String {
    "local-sha256-hash".into()
}
fn default_embedding_dim() -> usize {
    384
}
fn default_embedding_batch_size() -> usize {
    100
}
fn default_image_backend() -> String {
    "openai-compatible-image".into()
}
fn default_image_model() -> String {
    "dall-e-3".into()
}
fn default_image_max_concurrent() -> usize {
    2
}
fn default_image_width() -> u32 {
    1024
}
fn default_image_height() -> u32 {
    1024
}
fn default_queue_capacity() -> usize {
    100
}
fn default_event_bus_capacity() -> usize {
    1024
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            database_url: default_database_url(),
            data_dir: default_data_dir(),
            cache_dir: default_cache_dir(),
            temp_dir: default_temp_dir(),
            log_dir: default_log_dir(),
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
            request_options_json: None,
            retry_times: default_retry_times(),
            max_tokens: LlmMaxTokensConfig::default(),
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
            default_width: default_image_width(),
            default_height: default_image_height(),
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            queue_capacity: default_queue_capacity(),
            event_bus_capacity: default_event_bus_capacity(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_config_does_not_default_to_a_database_url() {
        assert_eq!(AppConfig::default().database_url, "");
    }

    #[test]
    fn app_config_defaults_to_offline_embedding() {
        let embedding = AppConfig::default().embedding;
        assert_eq!(embedding.backend, "local_hash");
        assert_eq!(embedding.model, "local-sha256-hash");
        assert_eq!(embedding.dimension, 384);
        assert!(embedding.base_url.is_empty());
        assert!(embedding.api_key.is_none());
    }
}
