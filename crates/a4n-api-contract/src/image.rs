use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListImagesByChapterInput {
    pub project_id: i64,
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ListVideoReferenceImagesInput {
    pub project_id: i64,
    pub chapter_id: i64,
    #[serde(default)]
    pub image_project_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedImageInput {
    #[serde(default)]
    pub prompt: Option<String>,
    #[serde(default)]
    pub image_type: Option<String>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub clear_chapter_id: bool,
    #[serde(default)]
    pub panel_id: Option<String>,
    #[serde(default)]
    pub clear_panel_id: bool,
    #[serde(default)]
    pub ai_score: Option<f32>,
    #[serde(default)]
    pub clear_ai_score: bool,
    #[serde(default)]
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UploadImageInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub image_type: String,
    pub prompt: String,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub panel_id: Option<String>,
    #[serde(default)]
    pub metadata_json: Option<Value>,
}
