#[cfg(test)]
use a4n_core::models::ai::{generated_image, generated_video};
#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::path::Path;

mod provider_images;
mod reference_images;
mod tail_frames;
mod types;
mod video_files;

pub(super) use self::reference_images::reference_image_ids;
pub(crate) use self::reference_images::{
    reference_image_preview_placeholder_by_id, reference_image_preview_placeholders,
    reference_image_preview_placeholders_by_ids, reference_image_provider_input_by_id,
    reference_image_provider_inputs, reference_image_provider_inputs_by_ids,
    remote_temp_upload_configured,
};
pub(crate) use self::tail_frames::{
    capture_video_tail_frame_asset, delete_video_tail_frame_assets,
};
pub(crate) use self::types::ProviderImageInputTransport;
pub(crate) use self::video_files::{
    append_secondary_error, best_effort_video_thumbnail_path, combine_optional_errors,
    ensure_chapter_videos_dir, ensure_ffmpeg, prompt_id_from_video, remove_partial_video_file,
    required_video_file_extension, run_ffmpeg_trim, save_video_bytes, trimmed_video_path,
    video_mime_from_path,
};

#[cfg(test)]
use self::provider_images::{
    encode_query_component, image_mime_from_path, is_provider_supported_image_content_type,
};
#[cfg(test)]
use self::reference_images::{
    reference_image_preview_notes, reference_image_preview_placeholder,
    reference_image_project_ids_in_reference_order,
};
#[cfg(test)]
use self::tail_frames::{tail_frame_image_belongs_to_video, video_shot_number};
#[cfg(test)]
use self::video_files::{
    chapter_tail_frame_images_dir, chapter_videos_dir, persist_video_thumbnail_path,
    required_trim_chapter_id,
};

#[cfg(test)]
mod tests;
