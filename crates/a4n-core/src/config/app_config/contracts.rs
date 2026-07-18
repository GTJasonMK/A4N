use super::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

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
