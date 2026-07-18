use super::prelude::*;

pub(super) const CHAPTER_VIDEO_TYPE: &str = "chapter_video";
pub(in crate::actions::video_records) const ASSEMBLY_INSERT_VIDEO_TYPE: &str = "assembly_insert";
pub(super) const ASSEMBLY_PUSH_VIDEO_TYPE: &str = "assembly_push";
pub(super) const ASSEMBLY_POSTPROCESS_EDIT_VIDEO_TYPE: &str = "assembly_postprocess_edit";
pub(super) const ASSEMBLY_ORDER_STEP: f64 = 1000.0;
pub(super) const ASSEMBLY_PUSH_STATUS_PENDING: &str = "pending";
pub(super) const ASSEMBLY_PUSH_STATUS_ACCEPTED: &str = "accepted";
pub(super) const ASSEMBLY_PUSH_STATUS_REJECTED: &str = "rejected";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledChapterVideoClipSummary {
    pub video_id: i64,
    pub video_type: String,
    pub shot_number: i32,
    pub sort_order: f64,
    pub insert_after_video_id: Option<i64>,
    pub duration_secs: f32,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssembledChapterVideoPreview {
    pub mime_type: String,
    pub data_base64: String,
    pub file_path: String,
    pub video_ids: Vec<i64>,
    pub clips: Vec<AssembledChapterVideoClipSummary>,
    pub duration_secs: f32,
    pub file_size: i64,
    pub updated_at: Option<String>,
    pub cached: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AssemblyPushResult {
    pub video: GeneratedVideoSummary,
    pub source_video_ids: Vec<i64>,
    pub push_index: i32,
    pub status: String,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct PushAssembledChapterVideoInput {
    #[serde(default)]
    pub end_video_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpdateAssemblyPushStatusInput {
    pub action: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ApplyAssemblyPostprocessEditInput {
    pub operation: String,
    #[serde(default)]
    pub at_secs: Option<f32>,
    #[serde(default)]
    pub start_secs: Option<f32>,
    #[serde(default)]
    pub end_secs: Option<f32>,
    #[serde(default)]
    pub source_video_id: Option<i64>,
    #[serde(default)]
    pub source_file_name: Option<String>,
    #[serde(default)]
    pub source_mime_type: Option<String>,
    #[serde(default)]
    pub source_data_base64: Option<String>,
}

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
    pub(super) output_path: PathBuf,
    pub(super) manifest_path: PathBuf,
    pub(super) manifest: AssemblyCacheManifest,
}

#[derive(Debug)]
pub(in crate::actions::video_records) struct AssemblyPushRow {
    pub(in crate::actions::video_records) video: generated_video::Model,
    pub(in crate::actions::video_records) status: String,
    pub(in crate::actions::video_records) push_index: i32,
    pub(in crate::actions::video_records) source_video_ids: Vec<i64>,
}

#[derive(Debug, Clone)]
pub(super) struct AssemblyInsertMetadata {
    pub(super) sort_order: f64,
    pub(super) insert_after_video_id: Option<i64>,
    pub(super) insert_after_shot_number: Option<i32>,
}

#[derive(Debug, Clone)]
pub(in crate::actions::video_records) struct PostprocessEditMetadata {
    pub(in crate::actions::video_records) push_video_id: i64,
    pub(in crate::actions::video_records) base_video_id: i64,
}

pub(in crate::actions::video_records) struct PostprocessEditSource {
    pub(super) path: PathBuf,
    pub(super) file_name: Option<String>,
    pub(super) source_video_id: Option<i64>,
    pub(super) _uploaded_source: Option<ImportedVideoSource>,
}
