use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use a4n_domain::{AssemblyPostprocessOperation, CanvasFrameKind};

use super::{GeneratedVideoSummary, VideoSegmentTarget};
use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

#[derive(Debug, Clone)]
pub struct AssemblyVideoRecords {
    pub workflow_mode: String,
    pub clips: Vec<AssemblyClipRecord>,
}

#[derive(Debug, Clone)]
pub struct AssemblyClipRecord {
    pub video_id: i64,
    pub video_type: String,
    pub shot_number: i32,
    pub sort_order: f64,
    pub insert_after_video_id: Option<i64>,
    pub file_path: String,
    pub duration_secs: f32,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct PreparedAssemblyPushRecord {
    pub video: GeneratedVideoSummary,
    pub source_video_ids: Vec<i64>,
    pub push_index: i32,
}

#[derive(Debug, Clone)]
pub struct VideoImportRecordSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub prompt_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct CanvasAssemblyPlan {
    pub sources: Vec<GeneratedVideoSummary>,
    pub title: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
}

#[derive(Debug, Clone)]
pub struct CanvasFramePlan {
    pub video: GeneratedVideoSummary,
    pub frame_kind: CanvasFrameKind,
    pub timestamp: f32,
}

#[derive(Debug, Clone)]
pub struct VideoTrimPlan {
    pub source: GeneratedVideoSummary,
    pub start_secs: f32,
    pub end_secs: f32,
    pub target_workflow_mode: String,
}

#[derive(Debug, Clone)]
pub struct PreparedAssemblyPostprocessEditRecord {
    pub video: GeneratedVideoSummary,
    pub base: GeneratedVideoSummary,
    pub operation: AssemblyPostprocessOperation,
    pub start_secs: f32,
    pub end_secs: f32,
}

/// Runtime supplies persisted file facts; the video owner commits the record.
#[derive(Debug, Clone)]
pub struct CompleteVideoRecordInput {
    pub record_id: i64,
    pub output_path: String,
    pub thumbnail_path: Option<String>,
    pub properties: a4n_domain::media::VideoFileProperties,
    pub prompt_id: Option<i64>,
    pub metadata_json: Option<Value>,
}

/// Result of committing a generated video record.
#[derive(Debug)]
pub struct VideoRecordCompletion {
    pub summary: GeneratedVideoSummary,
    pub adopted_authoritative: bool,
}

/// Stable failure surface for generated video record completion.
#[derive(Debug)]
pub enum VideoRecordCompletionError {
    Submission {
        error: a4n_error::AppError,
        authoritative: Option<VideoRecordCompletion>,
        lookup_error: Option<a4n_error::AppError>,
    },
}

/// Runtime-provided facts used to register a captured video tail frame.
#[derive(Debug, Clone)]
pub struct VideoTailFrameInput {
    pub video_id: i64,
    pub file_path: String,
    pub file_size: i64,
}

/// Persisted summary returned after registering a captured video tail frame.
#[derive(Debug, Clone)]
pub struct VideoTailFrameSummary {
    pub image_id: i64,
    pub file_path: String,
    pub shot_number: i32,
}

/// Input used by the video owner to prepare an assembly push record.
#[derive(Debug, Clone)]
pub struct AssemblyPushClipInput {
    pub video_id: i64,
    pub sort_order: f64,
    pub duration_secs: f32,
    pub width: i32,
    pub height: i32,
    pub fps: i32,
}

/// Owner-approved base and push records for an assembly postprocess edit.
#[derive(Debug, Clone)]
pub struct AssemblyPostprocessPlan {
    pub push: GeneratedVideoSummary,
    pub base: GeneratedVideoSummary,
    pub operation: AssemblyPostprocessOperation,
}

/// Source selected by the video owner for an assembly postprocess edit.
#[derive(Debug, Clone)]
pub enum AssemblyPostprocessSourcePlan {
    None,
    Existing(GeneratedVideoSummary),
    Upload,
}

/// Runtime-provided source facts for an assembly postprocess edit record.
#[derive(Debug, Clone)]
pub struct AssemblyPostprocessEditRecordInput {
    pub at_secs: Option<f32>,
    pub start_secs: Option<f32>,
    pub end_secs: Option<f32>,
    pub source_video_id: Option<i64>,
    pub source_file_name: Option<String>,
    pub source_duration_secs: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedVideoInput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "String")]
    pub prompt: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    #[schemars(with = "Option<String>")]
    pub negative_prompt: Option<Option<String>>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "bool")]
    pub is_favorite: Option<bool>,
    #[schemars(length(min = 1))]
    pub expected_updated_at: String,
}

impl UpdateGeneratedVideoInput {
    pub fn is_empty(&self) -> bool {
        self.prompt.is_none() && self.negative_prompt.is_none() && self.is_favorite.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GenerateChapterVideosBatchResult {
    pub videos: Vec<GeneratedVideoSummary>,
    pub success_count: usize,
    pub failed_count: usize,
    pub pending_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VideoScrubFramesData {
    pub data_base64: String,
    pub frames_per_second: u32,
    pub mime_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VideoPlaybackData {
    pub mime_type: String,
    pub data_base64: String,
    pub scrub_frames: Option<VideoScrubFramesData>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectSegmentVideoInput {
    #[schemars(range(min = 1))]
    pub segment_plan_prompt_id: i64,
    pub target_segment: VideoSegmentTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AssembledChapterVideoClipSummary {
    pub video_id: i64,
    pub video_type: String,
    pub shot_number: i32,
    pub sort_order: f64,
    pub insert_after_video_id: Option<i64>,
    pub duration_secs: f32,
    pub file_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AssemblyPushResult {
    pub video: GeneratedVideoSummary,
    pub source_video_ids: Vec<i64>,
    pub push_index: i32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PushAssembledChapterVideoInput {
    #[serde(default)]
    pub end_video_id: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AssemblyPushAction {
    Accept,
    Reject,
    Undo,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateAssemblyPushStatusInput {
    pub action: AssemblyPushAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ApplyAssemblyPostprocessEditInput {
    pub operation: AssemblyPostprocessOperation,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UploadChapterVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    pub prompt_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UploadReferenceMediaVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    #[schemars(range(min = 1))]
    pub segment_plan_prompt_id: i64,
    pub target_segment: VideoSegmentTarget,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UploadAssemblyInsertVideoInput {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
    #[serde(default)]
    pub insert_after_video_id: Option<i64>,
}
