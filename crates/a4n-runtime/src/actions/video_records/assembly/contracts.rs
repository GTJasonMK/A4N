use super::super::imports::ImportedVideoSource;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub use a4n_api_contract::video::{
    ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
    AssembledChapterVideoPreview, AssemblyPushResult, PushAssembledChapterVideoInput,
};

#[derive(Debug, Clone)]
pub(in crate::actions::video_records) struct AssemblyClip {
    pub(in crate::actions::video_records) video_id: i64,
    pub(in crate::actions::video_records) video_type: String,
    pub(in crate::actions::video_records) shot_number: i32,
    pub(in crate::actions::video_records) sort_order: f64,
    pub(in crate::actions::video_records) insert_after_video_id: Option<i64>,
    pub(in crate::actions::video_records) path: PathBuf,
    pub(in crate::actions::video_records) file_path: String,
    pub(in crate::actions::video_records) duration_secs: f32,
    pub(in crate::actions::video_records) width: i32,
    pub(in crate::actions::video_records) height: i32,
    pub(in crate::actions::video_records) fps: i32,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::actions::video_records) struct AssemblyCacheManifest {
    pub(in crate::actions::video_records) version: u32,
    pub(in crate::actions::video_records) project_id: i64,
    pub(in crate::actions::video_records) chapter_id: i64,
    pub(in crate::actions::video_records) workflow_mode: String,
    pub(in crate::actions::video_records) width: i32,
    pub(in crate::actions::video_records) height: i32,
    pub(in crate::actions::video_records) fps: i32,
    pub(in crate::actions::video_records) audio_enabled: bool,
    pub(in crate::actions::video_records) clips: Vec<AssemblyCacheClip>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(in crate::actions::video_records) struct AssemblyCacheClip {
    pub(in crate::actions::video_records) video_id: i64,
    pub(in crate::actions::video_records) video_type: String,
    pub(in crate::actions::video_records) shot_number: i32,
    pub(in crate::actions::video_records) sort_order: f64,
    pub(in crate::actions::video_records) insert_after_video_id: Option<i64>,
    pub(in crate::actions::video_records) file_path: String,
    pub(in crate::actions::video_records) file_size: u64,
    pub(in crate::actions::video_records) modified_unix_millis: u64,
    pub(in crate::actions::video_records) has_audio: bool,
    pub(in crate::actions::video_records) duration_secs: f32,
    pub(in crate::actions::video_records) width: i32,
    pub(in crate::actions::video_records) height: i32,
    pub(in crate::actions::video_records) fps: i32,
}

pub(super) struct AssemblyRequestContext {
    pub(super) clips: Vec<AssemblyClip>,
    pub(super) duration_secs: f32,
    pub(super) output_path: PathBuf,
    pub(super) manifest_path: PathBuf,
    pub(super) manifest: AssemblyCacheManifest,
}

pub(in crate::actions::video_records) struct PostprocessEditSource {
    pub(super) path: PathBuf,
    pub(super) file_name: Option<String>,
    pub(super) source_video_id: Option<i64>,
    pub(super) _uploaded_source: Option<ImportedVideoSource>,
}
