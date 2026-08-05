use serde_json::Value;

pub use a4n_video::{
    CLIPROXY_OPENAI_VIDEO_BACKEND, GEMINI_VEO_VIDEO_BACKEND, NEWAPI_VIDEO_BACKEND,
    OPENAI_CHAT_VIDEO_BACKEND, OPENAI_COMPATIBLE_VIDEO_BACKEND, SEEDANCE2_VIDEO_BACKEND,
};
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
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct VideoDurationCapabilityValues {
    pub(super) duration_mode: String,
    pub(super) min_duration_secs: i32,
    pub(super) max_duration_secs: i32,
    pub(super) supported_durations_secs: Vec<i32>,
}
