use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(in crate::video) struct ChatVideoPromptShot {
    pub shot_number: i32,
    pub title: String,
    pub scene_summary: String,
    pub chat_prompt: String,
    pub negative_prompt: String,
    pub chat_video: Value,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(super) struct ChatVideoPromptShotInput {
    pub title: String,
    pub scene_summary: String,
    pub chat_prompt: String,
    pub negative_prompt: String,
}

#[derive(Debug, Deserialize, schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub(in crate::video::prompts::json) struct ChatVideoPromptResponse {
    pub(super) shots: Vec<ChatVideoPromptShotInput>,
}
