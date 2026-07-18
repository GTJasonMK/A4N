use super::super::shared::deserialize_optional_string_update;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub(super) use super::mapping::{
    video_config_summary, video_create_record_input, video_update_record_input,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfigSummary {
    pub id: i64,
    pub name: String,
    pub backend: String,
    pub base_url: Option<String>,
    pub api_key_encrypted: Option<String>,
    pub has_api_key: bool,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
    pub is_default: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VideoConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    pub api_key_encrypted: Option<String>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoConfigInput {
    pub name: String,
    pub backend: String,
    pub base_url: String,
    #[serde(default, deserialize_with = "deserialize_optional_string_update")]
    pub api_key_encrypted: Option<Option<String>>,
    pub model: String,
    pub max_concurrent: i32,
    pub default_width: i32,
    pub default_height: i32,
    pub default_duration_secs: i32,
    pub min_duration_secs: i32,
    pub max_duration_secs: i32,
    pub duration_mode: String,
    pub supported_durations_json: Option<Value>,
    pub default_fps: i32,
    pub provider_options_json: Option<Value>,
}
