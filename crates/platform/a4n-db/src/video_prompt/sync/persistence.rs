mod records;
mod trace;

pub(super) use records::{prompt_active_model, replace_prompt_from_sync_draft};
pub(super) use trace::record_chapter_video_prompt_generation_run;
