use a4n_core::config::app_config::LlmMaxTokensConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeConfigSnapshot {
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub llm: Option<RuntimeLlmConfig>,
    #[serde(default)]
    pub llm_configs: Vec<RuntimeLlmConfig>,
    #[serde(default)]
    pub image: Option<RuntimeImageConfig>,
    #[serde(default)]
    pub image_configs: Vec<RuntimeImageConfig>,
    #[serde(default)]
    pub video: Option<RuntimeVideoConfig>,
    #[serde(default)]
    pub video_configs: Vec<RuntimeVideoConfig>,
    #[serde(default)]
    pub embedding: Option<RuntimeEmbeddingConfig>,
    #[serde(default)]
    pub llm_default_source: Option<String>,
    #[serde(default)]
    pub image_default_source: Option<String>,
    #[serde(default)]
    pub video_default_source: Option<String>,
    #[serde(default)]
    pub system_llm_api_key_encrypted: Option<String>,
    #[serde(default)]
    pub system_image_api_key_encrypted: Option<String>,
    #[serde(default)]
    pub system_video_api_key_encrypted: Option<String>,
    #[serde(default)]
    pub llm_max_tokens: Option<LlmMaxTokensConfig>,
    #[serde(default)]
    pub client_context: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeLlmConfig {
    pub id: Option<i64>,
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(default)]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    #[serde(default)]
    pub request_options_json: Option<Value>,
    #[serde(default)]
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeImageConfig {
    pub id: Option<i64>,
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeVideoConfig {
    pub id: Option<i64>,
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    #[serde(default)]
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    #[serde(default)]
    pub is_default: bool,
    #[serde(default)]
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuntimeEmbeddingConfig {
    pub id: Option<i64>,
    pub name: String,
    pub backend: String,
    #[serde(default)]
    pub base_url: Option<String>,
    #[serde(default)]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    #[serde(default)]
    pub local_model_path: Option<String>,
    #[serde(default)]
    pub is_default: bool,
}
