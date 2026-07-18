mod deletion;
mod persistence;
mod rules;
mod update;

pub use deletion::{delete_chapter_video_prompt_record, delete_chapter_video_prompt_records};
pub use persistence::{insert_chapter_video_prompt_record, replace_chapter_video_prompt_record};
pub use update::{update_chapter_video_prompt_metadata_record, update_chapter_video_prompt_record};

#[cfg(test)]
pub(super) use rules::{
    ensure_chapter_video_prompt_not_running, validate_chapter_video_prompt_duration,
};
