use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoPlaybackData {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteChapterVideoAssetsResult {
    pub deleted_video_count: u64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedVideoInput {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub clear_negative_prompt: bool,
    #[serde(default)]
    pub metadata_json: Option<Value>,
    #[serde(default)]
    pub clear_metadata_json: bool,
    #[serde(default)]
    pub is_favorite: Option<bool>,
    #[serde(default)]
    pub insert_after_video_id: Option<i64>,
    #[serde(default)]
    pub move_assembly_insert_to_start: bool,
}
