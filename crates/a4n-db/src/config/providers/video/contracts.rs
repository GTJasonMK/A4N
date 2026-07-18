use serde_json::Value;

#[derive(Clone, Debug)]
pub struct VideoConfigCreateRecordInput {
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

#[derive(Clone, Debug)]
pub struct VideoConfigUpdateRecordInput {
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
