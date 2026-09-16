use serde_json::Value;

mod create;
mod deletion;
mod lifecycle;
mod records;

pub use create::{
    create_generated_video_record, create_generated_video_record_with_generation_run,
    create_prompt_generated_video_record_with_generation_run,
};
pub(crate) use deletion::clear_generated_video_references_by_ids;
pub use deletion::{
    delete_generated_video_record_with_cleanup, delete_missing_generated_video_references_record,
};
pub use lifecycle::{
    GeneratedVideoErrorStatusBatchInput, mark_generated_video_completed_and_link_prompt_record,
    mark_generated_video_error_status_and_prompt_record,
    mark_generated_video_job_and_prompt_record,
    mark_generated_videos_error_status_and_prompt_records,
    recover_stale_generated_videos_without_provider_job,
    update_generated_video_waiting_status_and_prompt_record,
};
pub use records::{
    generated_video_by_id, generated_videos_by_ids, generated_videos_by_provider_job_id,
    generated_videos_for_chapter, merge_video_provider_response_metadata,
    missing_video_reference_project_ids, update_generated_video_duration_record,
    update_generated_video_record, validate_video_metadata_update_preserves_system_fields,
};

#[derive(Debug, Clone)]
pub(in crate::video::generated_videos) struct CreateGeneratedVideoRecordInput {
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

#[derive(Debug, Clone, Default)]
pub struct UpdateGeneratedVideoRecordInput {
    pub prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub clear_negative_prompt: bool,
    pub metadata_json: Option<Value>,
    pub clear_metadata_json: bool,
    pub is_favorite: Option<bool>,
}
