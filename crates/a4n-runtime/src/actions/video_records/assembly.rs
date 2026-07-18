mod prelude {
    pub(super) use super::super::imports::ImportedVideoSource;
    pub(super) use crate::actions::video_generation::GeneratedVideoSummary;
    pub(super) use a4n_core::models::ai::generated_video;
    pub(super) use serde::{Deserialize, Serialize};
    pub(super) use std::path::PathBuf;
}

mod actions;
mod cache;
mod clips;
mod contracts;
mod postprocess;
mod push;

#[cfg(test)]
pub(in crate::actions::video_records) use actions::persist_assembly_preview_cache;
pub use actions::{
    apply_assembly_postprocess_edit, assemble_chapter_videos,
    load_cached_assembled_chapter_video_preview, push_assembled_chapter_video,
    update_assembly_push_status,
};
pub use contracts::{
    ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
    AssembledChapterVideoPreview, AssemblyPushResult, PushAssembledChapterVideoInput,
    UpdateAssemblyPushStatusInput,
};

#[cfg(test)]
pub(in crate::actions::video_records) use cache::build_assembly_cache_manifest_with_audio_probe;
pub(in crate::actions::video_records) use cache::run_ffmpeg_assemble_videos;
#[cfg(test)]
pub(in crate::actions::video_records) use cache::{
    assembly_cache_matches, write_assembly_cache_manifest,
};
#[cfg(test)]
pub(in crate::actions::video_records) use cache::{assembly_dimensions, assembly_fps};
#[cfg(test)]
pub(in crate::actions::video_records) use clips::{
    assembly_clip_shot_number, assembly_clip_sort_order, assembly_insert_metadata_with_position,
};
pub(in crate::actions::video_records) use clips::{
    assembly_clips, is_assembly_clip_video_type, next_assembly_insert_sort_order,
    resolve_assembly_insert_position_metadata,
};
pub(in crate::actions::video_records) use contracts::{
    ASSEMBLY_INSERT_VIDEO_TYPE, AssemblyCacheClip, AssemblyCacheManifest, AssemblyClip,
};
#[cfg(test)]
pub(in crate::actions::video_records) use postprocess::parse_postprocess_edit_metadata;
#[cfg(test)]
pub(in crate::actions::video_records) use postprocess::{
    postprocess_edit_range, validate_postprocess_operation,
};
#[cfg(test)]
pub(in crate::actions::video_records) use push::{
    active_assembly_push_source_ids, ensure_active_pushes_match_clip_prefix,
    selected_assembly_push_source_clips,
};
pub(in crate::actions::video_records) use push::{
    assembly_push_rows, ensure_insert_position_not_inside_pushed_prefix,
    pushed_source_ids_for_chapter, validate_assembly_workflow_mode,
};
