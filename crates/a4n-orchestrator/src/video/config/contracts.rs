use serde_json::Value;

pub(super) const OPENAI_COMPATIBLE_VIDEO_BACKEND: &str = "openai-compatible-video";
pub(super) const NEWAPI_VIDEO_BACKEND: &str = "newapi-video";
pub(super) const GEMINI_VEO_VIDEO_BACKEND: &str = "gemini-veo-video";
pub(super) const SEEDANCE2_VIDEO_BACKEND: &str = "seedance2-video";
pub(super) const OPENAI_CHAT_VIDEO_BACKEND: &str = "openai-chat-video";
pub(super) const CLIPROXY_OPENAI_VIDEO_BACKEND: &str = "cliproxy-openai-video";
pub(super) const VIDEO_DURATION_MODE_RANGE: &str = "range";
pub(super) const VIDEO_DURATION_MODE_FIXED: &str = "fixed";

#[derive(Debug, Clone)]
pub struct VideoConfigParams {
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

#[derive(Debug, Clone)]
pub struct UpdateVideoConfigParams {
    pub name: String,
    pub backend: String,
    pub base_url: String,
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VideoDurationCapabilityValues {
    pub(super) duration_mode: String,
    pub(super) min_duration_secs: i32,
    pub(super) max_duration_secs: i32,
    pub(super) supported_durations_secs: Vec<i32>,
}
