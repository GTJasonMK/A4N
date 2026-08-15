#[cfg(test)]
use a4n_core::models::ai::{generated_image, generated_video};
#[cfg(test)]
use std::path::Path;

mod provider_images;
mod reference_images;
mod tail_frames;
mod types;
mod video_files;

pub(crate) use self::provider_images::{
    RemoteTempMediaDeleteOutcome, delete_remote_temp_media_artifact,
    media_bytes_to_remote_temp_media_url,
};
#[cfg(test)]
pub(super) use self::reference_images::reference_image_ids;
pub(crate) use self::reference_images::{
    reference_image_provider_input_by_id, reference_image_provider_inputs_by_ids,
    reference_image_request_preview_url_by_id, reference_image_request_preview_urls_by_ids,
    remote_temp_upload_config, remote_temp_upload_configured,
};
pub(crate) use self::tail_frames::{
    capture_video_tail_frame_asset, delete_video_tail_frame_assets,
};
pub(crate) use self::video_files::{
    append_secondary_error, best_effort_video_thumbnail_path, combine_optional_errors,
    ensure_chapter_videos_dir, ensure_ffmpeg, persist_canvas_video_bytes, prompt_id_from_video,
    remove_partial_video_file, run_ffmpeg_trim, save_video_bytes, trimmed_video_path,
    video_mime_from_path,
};

#[cfg(test)]
use self::video_files::{
    chapter_tail_frame_images_dir, chapter_videos_dir, persist_video_thumbnail_path,
    required_trim_chapter_id, required_video_file_extension,
};
#[cfg(test)]
mod tests;
