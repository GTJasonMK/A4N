use serde_json::Value;

mod create;
mod deletion;
mod lifecycle;
mod records;

pub use create::*;
pub(crate) use deletion::clear_generated_video_references_record;
pub use deletion::{
    delete_generated_video_record, delete_missing_generated_video_references_record,
};
pub use lifecycle::*;
pub use records::*;

#[derive(Debug, Clone)]
pub struct CreateGeneratedVideoRecordInput {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: String,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub metadata_json: Option<Value>,
}

/// 业务侧创建草稿：`workflow_mode` 可选，写库前由 `create_generated_video_record` 校验规范化。
#[derive(Debug, Clone)]
pub struct CreateGeneratedVideoRecordDraftInput {
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub metadata_json: Option<Value>,
}

#[derive(Debug, Clone, Default)]
pub struct UpdateGeneratedVideoRecordInput {
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub clear_negative_prompt: bool,
    pub metadata_json: Option<Value>,
    pub clear_metadata_json: bool,
    pub is_favorite: Option<bool>,
}
