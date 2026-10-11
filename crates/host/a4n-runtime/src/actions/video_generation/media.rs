#[cfg(test)]
use a4n_db_models::models::ai::{generated_image, generated_video};
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
pub(crate) use self::reference_images::{
    ReferenceImageProviderContext, reference_image_provider_input_by_id,
    reference_image_provider_inputs_by_ids, reference_image_request_preview_url_by_id,
    reference_image_request_preview_urls_by_ids, remote_temp_upload_config,
};
pub(crate) use self::tail_frames::{
    capture_video_tail_frame_asset, delete_video_tail_frame_assets,
};
pub(crate) use self::video_files::{
    best_effort_video_thumbnail_path, ensure_chapter_videos_dir, ensure_project_videos_dir,
    persist_canvas_video_bytes, remove_partial_video_file, run_ffmpeg_trim, save_video_bytes,
    trimmed_video_path,
};

#[cfg(test)]
use self::video_files::{
    chapter_tail_frame_images_dir, chapter_videos_dir, persist_video_thumbnail_path,
    required_video_file_extension,
};
#[cfg(test)]
mod tests;
