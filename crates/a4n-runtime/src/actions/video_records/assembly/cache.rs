mod manifest;
mod paths;
mod render;
mod storage;

pub(super) use manifest::build_assembly_cache_manifest;
#[cfg(test)]
pub(in crate::actions::video_records) use manifest::build_assembly_cache_manifest_with_audio_probe;
pub(in crate::actions::video_records) use manifest::{assembly_dimensions, assembly_fps};
pub(super) use paths::{assembled_video_output_path, assembly_cache_manifest_path};
pub(in crate::actions::video_records) use render::run_ffmpeg_assemble_videos;
pub(in crate::actions::video_records) use storage::{
    assembly_cache_matches, write_assembly_cache_manifest,
};
