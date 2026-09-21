mod deletion;
mod persistence;
mod rules;
mod update;

pub(crate) use deletion::delete_chapter_video_prompt_records;
pub(crate) use persistence::replace_chapter_video_prompt_record;
pub(crate) use rules::ensure_chapter_video_prompt_not_running;
pub(crate) use update::update_chapter_video_prompt_metadata_record;
pub use update::update_chapter_video_prompt_record;

#[cfg(test)]
pub(super) use rules::validate_chapter_video_prompt_duration;
