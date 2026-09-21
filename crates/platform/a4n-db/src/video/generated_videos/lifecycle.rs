mod completion;
mod context;
mod error;
mod job;
mod waiting;

pub use completion::mark_generated_video_completed_and_link_prompt_record;
pub use error::{
    mark_generated_video_error_status_and_prompt_record,
    mark_generated_videos_error_status_and_prompt_records,
    recover_stale_generated_videos_without_provider_job,
};
pub use job::mark_generated_video_job_and_prompt_record;
pub use waiting::update_generated_video_waiting_status_and_prompt_record;

#[cfg(test)]
mod tests;
