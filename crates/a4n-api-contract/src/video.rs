use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::generation_tasks::GenerationStartRequestMeta;

/// 视频生产的唯一启动合同。
///
/// 查询、上传、预览和 provider 轮询继续使用各自合同；生产入口统一经过
/// generation reservation/session/worker。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartVideoGenerationInput {
    pub meta: GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    pub operation: VideoGenerationOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VideoGenerationOperation {
    Chapter {
        #[schemars(range(min = 1))]
        chapter_id: i64,
        #[schemars(range(min = 1))]
        prompt_id: i64,
        #[schemars(range(min = 1))]
        config_id: Option<i64>,
    },
    ChapterBatch {
        #[schemars(range(min = 1, max = 10_000))]
        prompt_ids: Vec<i64>,
        #[schemars(range(min = 1))]
        config_id: Option<i64>,
        #[schemars(range(min = 1))]
        chapter_id: i64,
    },
    ReferenceMedia {
        #[schemars(range(min = 1))]
        chapter_id: i64,
        input: ReferenceMediaVideoInput,
    },
}

/// 参考媒体视频请求；预览和正式生产共用这一 DTO。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMediaVideoInput {
    /// 关联的片段视频提示词。预览与生成使用同一片段身份。
    #[schemars(range(min = 1))]
    pub segment_prompt_id: i64,
    /// 片段顺序快照，用于结果筛选和身份校验。
    #[schemars(range(min = 1))]
    pub shot_number: i32,
    #[schemars(length(max = 100_000))]
    pub prompt: String,
    #[serde(default)]
    pub image_files: Vec<String>,
    #[serde(default)]
    pub image_uploads: Vec<ReferenceMediaUploadInput>,
    #[serde(default)]
    pub video_files: Vec<String>,
    #[serde(default)]
    pub video_uploads: Vec<ReferenceMediaUploadInput>,
    #[serde(default)]
    pub audio_files: Vec<String>,
    #[serde(default)]
    pub audio_uploads: Vec<ReferenceMediaUploadInput>,
    #[schemars(range(min = 1, max = 60))]
    pub duration_secs: i32,
    #[schemars(length(max = 16))]
    pub aspect_ratio: String,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub config_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMediaUploadInput {
    #[schemars(length(min = 1, max = 255))]
    pub file_name: String,
    #[schemars(length(min = 1, max = 128))]
    pub mime_type: String,
    #[schemars(length(min = 1))]
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GeneratedVideoSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,
    pub video_type: String,
    pub workflow_mode: Option<String>,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub prompt: String,
    pub negative_prompt: Option<String>,
    pub model: String,
    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,
    pub file_size: Option<i64>,
    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,
    pub status: String,
    pub provider_job_id: Option<String>,
    pub error_message: Option<String>,
    pub metadata_json: Option<Value>,
    pub is_favorite: bool,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct GenerateChapterVideosBatchResult {
    pub videos: Vec<GeneratedVideoSummary>,
    pub success_count: usize,
    pub failed_count: usize,
    pub pending_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VideoPlaybackData {
    pub mime_type: String,
    pub data_base64: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct DeleteChapterVideoAssetsResult {
    pub deleted_video_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SelectSegmentVideoInput {
    #[schemars(range(min = 1))]
    pub shot_number: i32,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub struct PushAssembledChapterVideoInput {
    #[serde(default)]
    pub end_video_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
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
    pub prompt: String,
    pub duration_secs: i32,
    pub aspect_ratio: String,
    #[schemars(range(min = 1))]
    pub segment_prompt_id: i64,
    #[schemars(range(min = 1))]
    pub shot_number: i32,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ImportCanvasVideoInput {
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub file_name: Option<String>,
    #[serde(default)]
    pub mime_type: Option<String>,
    #[serde(default)]
    pub data_base64: Option<String>,
    #[serde(default)]
    pub prompt: Option<String>,
}
