use sea_orm::FromQueryResult;
use serde_json::Value;

#[derive(Debug, Clone, FromQueryResult, PartialEq)]
pub struct ChapterVideoPromptTimelineRecord {
    pub id: i64,
    pub generated_video_id: Option<i64>,
    pub title: Option<String>,
    pub scene_summary: Option<String>,
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
}

#[derive(Debug, Clone, FromQueryResult, PartialEq)]
pub(crate) struct PromptDependencyRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: Option<String>,
    pub prompt_type: String,
    pub workflow_mode: String,
    pub aspect_ratio: Option<String>,
    pub metadata_json: Option<Value>,
}

/// 只用于视频提示词候选链匹配的窄读模型。
#[derive(Debug, Clone, FromQueryResult, PartialEq)]
pub struct ChapterVideoPromptCandidateRecord {
    pub id: i64,
    pub prompt_type: String,
    pub workflow_mode: String,
    pub shot_number: i32,
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateChapterVideoPromptRecordInput {
    pub visual_prompt: Option<String>,
    pub motion_prompt: Option<Option<String>>,
    pub negative_prompt: Option<Option<String>>,
    pub duration_secs: Option<i32>,
    pub reference_image_ids_json: Option<Option<Value>>,
    pub metadata_json: Option<Value>,
    pub reset_video_binding_to_pending: bool,
}

#[derive(Debug, Clone)]
pub struct ChapterVideoPromptRecordFields {
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: Option<String>,
    pub prompt_type: String,
    pub workflow_mode: String,
    pub shot_number: i32,
    pub title: Option<String>,
    pub scene_summary: Option<String>,
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub duration_secs: i32,
    pub aspect_ratio: Option<String>,
    pub reference_image_ids_json: Option<Value>,
    pub metadata_json: Option<Value>,
}
