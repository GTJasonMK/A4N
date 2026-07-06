use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AllConfigsExport {
    pub version: String,
    pub export_time: String,
    pub export_type: String,

    pub llm_configs: Vec<LlmRow>,
    pub embedding_configs: Vec<EmbeddingRow>,
    pub image_configs: Vec<ImageRow>,
    pub video_configs: Vec<VideoRow>,
    pub prompts_modified: Vec<PromptRow>,
    pub themes: Vec<ThemeRow>,
    pub system_settings: Vec<SystemSettingRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRow {
    pub name: String,
    pub provider: String,
    pub base_url: String,
    #[serde(deserialize_with = "deserialize_required_optional_string")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub timeout_secs: i32,
    pub max_context_tokens: i32,
    pub max_output_tokens: i32,
    #[serde(default)]
    pub request_options_json: Option<serde_json::Value>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRow {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    #[serde(deserialize_with = "deserialize_required_optional_string")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub dimension: i32,
    pub batch_size: i32,
    pub local_model_path: Option<String>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageRow {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    #[serde(deserialize_with = "deserialize_required_optional_string")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    #[serde(default)]
    pub provider_options_json: Option<serde_json::Value>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoRow {
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    #[serde(deserialize_with = "deserialize_required_optional_string")]
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<serde_json::Value>,
    pub default_fps: i32,
    #[serde(default)]
    pub provider_options_json: Option<serde_json::Value>,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptRow {
    pub name: String,
    pub content: String,
    pub title: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeRow {
    pub user_id: Option<i64>,
    pub name: String,
    pub mode: String,
    pub css_variables: serde_json::Value,
    pub is_system_default: bool,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSettingRow {
    pub key: String,
    pub value: serde_json::Value,
    pub category: String,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ImportAllConfigsInput {
    pub settings_json: String,
    /// 同名/同 key 时是否覆盖现有。
    pub overwrite: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportAllResult {
    pub success: bool,
    pub message: String,
    pub details: Vec<String>,
}

fn deserialize_required_optional_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<String>::deserialize(deserializer)
}
