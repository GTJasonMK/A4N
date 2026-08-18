mod ffmpeg;
mod files;
mod paths;
mod thumbnails;

pub(super) use self::ffmpeg::capture_video_tail_frame;
pub(crate) use self::ffmpeg::run_ffmpeg_trim;
#[cfg(test)]
pub(super) use self::files::required_video_file_extension;
pub(crate) use self::files::{
    append_secondary_error, combine_optional_errors, persist_canvas_video_bytes,
    prompt_id_from_video, remove_partial_video_file, save_video_bytes,
};
pub(super) use self::paths::video_tail_frame_path;
#[cfg(test)]
pub(super) use self::paths::{
    chapter_tail_frame_images_dir, chapter_videos_dir,
};
pub(crate) use self::paths::{
    ensure_chapter_videos_dir, ensure_project_videos_dir, trimmed_video_path,
};
pub(crate) use self::thumbnails::best_effort_video_thumbnail_path;
#[cfg(test)]
pub(super) use self::thumbnails::persist_video_thumbnail_path;
