mod prelude {
    pub(super) use super::super::assembly::{
        AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip, run_ffmpeg_assemble_videos,
    };
    pub(super) use crate::AppState;
    pub(super) use crate::actions::access;
    pub(super) use crate::actions::image_generation::{GeneratedImageSummary, to_summary};
    pub(super) use crate::actions::video_generation::media::{
        append_secondary_error, best_effort_video_thumbnail_path, capture_video_tail_frame_asset,
        combine_optional_errors, delete_video_tail_frame_assets, ensure_chapter_videos_dir,
        ensure_ffmpeg, remove_partial_video_file, run_ffmpeg_trim, trimmed_video_path,
    };
    pub(super) use crate::actions::video_generation::video_to_summary;
    pub(super) use crate::actions::video_generation::{
        GeneratedVideoSummary, reference_media_dimensions_for_aspect_ratio,
    };
    pub(super) use crate::actions::video_prompts::{
        ensure_prompt_matches_current_chapter, resolve_project_chapter_video_source,
    };
    pub(super) use crate::file_utils::{
        canonicalize_data_file_path, data_dir_relative_file_path, deletable_data_file_path,
        persist_created_media_file, remove_created_file_on_error_sync,
    };
    pub(super) use a4n_core::files::paths::ProjectPaths;
    pub(super) use a4n_core::models::ai::generated_video;
    pub(super) use a4n_core::models::chapter::chapter_video_prompt;
    pub(super) use a4n_orchestrator::video::{
        CreateGeneratedImageRecordDraftInput as CreateImageRecord,
        CreateGeneratedVideoRecordDraftInput as CreateVideoRecord,
    };
    pub(super) use base64::Engine;
    pub(super) use chrono::Utc;
    pub(super) use serde::Deserialize;
    pub(super) use serde_json::{Value, json};
    pub(super) use std::future::Future;
    pub(super) use std::io::ErrorKind;
    pub(super) use std::path::{Path, PathBuf};
}

mod canvas;
mod chapter;
mod contracts;
mod processing;
mod reference;
mod source;

pub use canvas::{assemble_canvas_videos, extract_canvas_video_frame, import_canvas_video};
pub use chapter::upload_chapter_video;
pub(in crate::actions::video_records) use contracts::ImportedVideoSource;
pub use contracts::{
    ImportCanvasVideoInput, UploadAssemblyInsertVideoInput, UploadChapterVideoInput,
    UploadReferenceMediaVideoInput,
};
pub use processing::trim_video;
pub use reference::{upload_assembly_insert_video, upload_reference_media_video};

pub(in crate::actions::video_records) use processing::imported_chapter_video_metadata;
#[cfg(test)]
pub(in crate::actions::video_records) use processing::{
    persist_trimmed_video_output, trimmed_video_metadata, validate_video_trim_range,
};
#[cfg(test)]
pub(in crate::actions::video_records) use source::validate_reference_media_import_options;
#[cfg(test)]
pub(in crate::actions::video_records) use source::{
    copy_imported_video_file, import_video_extension, import_video_source_path,
    persist_uploaded_video_source, video_probe_error_message,
};
pub(in crate::actions::video_records) use source::{
    delete_video_record_error, delete_video_record_then_files, import_video_output_path,
    mark_imported_video_completed, probe_video_properties, uploaded_video_source,
    video_delete_file_paths,
};
use source::{import_video_prompt, imported_video_prompt_text, persist_imported_video_file};
