mod metadata;
mod mutations;
mod prompt_context;
mod prompt_lifecycle;
mod queries;
mod references;

pub use metadata::{
    merge_video_provider_response_metadata, validate_video_metadata_update_preserves_system_fields,
};
pub use mutations::{update_generated_video_duration_record, update_generated_video_record};
pub(in crate::video::generated_videos) use prompt_context::prompt_for_generated_video_record;
pub(in crate::video::generated_videos) use prompt_lifecycle::mark_prompt_video_generation_running_for_video_record;
pub use queries::{
    generated_video_by_id, generated_videos_by_provider_job_id, generated_videos_for_chapter,
};
pub use references::missing_video_reference_project_ids;
