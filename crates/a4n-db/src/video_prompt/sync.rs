use super::*;

mod identity;
mod orchestration;
mod persistence;
mod preservation;
mod validation;

pub use orchestration::sync_chapter_video_prompts_with_generation_runs;

pub(in crate::video_prompt) fn validate_prompt_sync_draft(
    project_id: i64,
    chapter_id: i64,
    draft: &ChapterVideoPromptSyncDraft,
) -> Result<(), VideoDbError> {
    validation::validate_prompt_sync_draft(project_id, chapter_id, draft)
}

pub(in crate::video_prompt) fn generated_prompt_status(prompt_type: &str) -> String {
    match prompt_type {
        chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN => {
            chapter_video_prompt::STATUS_COMPLETED.to_string()
        }
        _ => chapter_video_prompt::STATUS_PENDING.to_string(),
    }
}
