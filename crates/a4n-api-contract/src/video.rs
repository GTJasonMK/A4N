use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_domain::DownstreamConflictStrategy;

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

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
    PromptSegmentPlan {
        #[schemars(range(min = 1))]
        chapter_id: i64,
        options: VideoPromptGenerationOptions,
    },
    PromptFrames {
        #[schemars(range(min = 1))]
        chapter_id: i64,
        options: VideoPromptGenerationOptions,
    },
    PromptVideos {
        #[schemars(range(min = 1))]
        chapter_id: i64,
        options: VideoPromptGenerationOptions,
    },
}

impl VideoGenerationOperation {
    pub fn is_prompt_generation(&self) -> bool {
        matches!(
            self,
            Self::PromptSegmentPlan { .. } | Self::PromptFrames { .. } | Self::PromptVideos { .. }
        )
    }
}

/// 章节视频提示词三段式生成共用的输入。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoPromptGenerationOptions {
    #[serde(default)]
    pub workflow_mode: Option<String>,
    /// 当前共享片段规划。生成片段规划时为空；生成某个片段的帧或最终提示词时必填。
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub segment_plan_prompt_id: Option<i64>,
    pub style: String,
    #[serde(default)]
    pub prompt_output_language: Option<String>,
    #[schemars(range(min = 1))]
    pub prompt_batch_size: i32,
    #[serde(default)]
    pub target_segment: Option<VideoSegmentTarget>,
    /// 生成共享片段规划时，可只规划已完成的某个拍摄场景。
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub target_narrative_unit_index: Option<u32>,
    #[schemars(range(min = 1, max = 60))]
    pub duration_secs: i32,
    #[serde(default)]
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
    #[serde(default)]
    pub video_library_image_ids: Vec<i64>,
    #[serde(default)]
    pub regenerate_existing: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentTarget {
    #[schemars(length(min = 1))]
    pub source_shot_id: String,
    #[schemars(range(min = 1))]
    pub segment_index_in_shot: i32,
}

/// 参考媒体视频请求；提示词、URL 槽、时长和画幅由当前片段 recipe 决定。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReferenceMediaVideoInput {
    /// 当前共享片段规划。预览与生成使用同一规划身份。
    #[schemars(range(min = 1))]
    pub segment_plan_prompt_id: i64,
    pub target_segment: VideoSegmentTarget,
    #[serde(default)]
    pub image_uploads: Vec<ReferenceMediaUploadInput>,
    #[serde(default)]
    pub video_uploads: Vec<ReferenceMediaUploadInput>,
    #[serde(default)]
    pub audio_uploads: Vec<ReferenceMediaUploadInput>,
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
pub struct ChapterVideoPromptSummary {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub style: Option<String>,
    pub prompt_type: String,
    pub workflow_mode: String,
    pub shot_number: i32,
    pub title: Option<String>,
    pub scene_summary: Option<String>,
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
    pub duration_secs: i32,
    pub aspect_ratio: Option<String>,
    pub reference_image_ids_json: Option<Value>,
    pub status: String,
    pub generated_video_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreviewChapterVideoPromptOverrideInput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "String")]
    pub visual_prompt: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    #[schemars(with = "Option<String>")]
    pub motion_prompt: Option<Option<String>>,
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
    #[schemars(with = "i32")]
    pub duration_secs: Option<i32>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    #[schemars(with = "Option<Value>")]
    pub reference_image_ids_json: Option<Option<Value>>,
}

impl PreviewChapterVideoPromptOverrideInput {
    pub fn is_empty(&self) -> bool {
        self.visual_prompt.is_none()
            && self.motion_prompt.is_none()
            && self.negative_prompt.is_none()
            && self.duration_secs.is_none()
            && self.reference_image_ids_json.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterVideoPromptInput {
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_non_null_update_field"
    )]
    #[schemars(with = "String")]
    pub visual_prompt: Option<String>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "deserialize_nullable_update_field"
    )]
    #[schemars(with = "Option<String>")]
    pub motion_prompt: Option<Option<String>>,
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
    #[schemars(with = "i32")]
    pub duration_secs: Option<i32>,
    pub downstream_strategy: DownstreamConflictStrategy,
}

impl UpdateChapterVideoPromptInput {
    pub fn is_empty(&self) -> bool {
        self.visual_prompt.is_none()
            && self.motion_prompt.is_none()
            && self.negative_prompt.is_none()
            && self.duration_secs.is_none()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizeChapterVideoPromptInput {
    #[schemars(length(min = 1))]
    pub instruction: String,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteChapterVideoPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    pub downstream_strategy: DownstreamConflictStrategy,
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
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateGeneratedVideoInput {
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "String")]
    pub prompt: Option<String>,
    #[serde(default, deserialize_with = "deserialize_nullable_update_field")]
    #[schemars(with = "Option<String>")]
    pub negative_prompt: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_non_null_update_field")]
    #[schemars(with = "bool")]
    pub is_favorite: Option<bool>,
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

#[cfg(test)]
mod tests {
    use super::{
        DeleteChapterVideoPromptInput, DownstreamConflictStrategy, OptimizeChapterVideoPromptInput,
        PreviewChapterVideoPromptOverrideInput, UpdateChapterVideoPromptInput,
        UpdateGeneratedVideoInput,
    };
    use serde_json::json;

    #[test]
    fn generated_video_update_has_one_strict_patch_contract() {
        let input: UpdateGeneratedVideoInput = serde_json::from_value(json!({
            "prompt": "new prompt",
            "negative_prompt": null,
            "is_favorite": true
        }))
        .expect("valid patch");
        assert_eq!(input.prompt.as_deref(), Some("new prompt"));
        assert_eq!(input.negative_prompt, Some(None));
        assert_eq!(input.is_favorite, Some(true));
        assert!(!input.is_empty());

        for value in [
            json!({"prompt": null}),
            json!({"is_favorite": null}),
            json!({"clear_negative_prompt": true}),
            json!({"metadata_json": {}}),
            json!({"clear_metadata_json": true}),
            json!({"insert_after_video_id": 1}),
            json!({"move_assembly_insert_to_start": true}),
        ] {
            assert!(serde_json::from_value::<UpdateGeneratedVideoInput>(value).is_err());
        }

        let empty: UpdateGeneratedVideoInput = serde_json::from_value(json!({})).unwrap();
        assert!(empty.is_empty());
    }

    #[test]
    fn chapter_video_prompt_mutations_have_one_strict_contract() {
        let update: UpdateChapterVideoPromptInput = serde_json::from_value(json!({
            "visual_prompt": "new visual",
            "motion_prompt": null,
            "negative_prompt": null,
            "duration_secs": 6,
            "downstream_strategy": "keep_downstream"
        }))
        .expect("valid prompt update");
        assert_eq!(update.visual_prompt.as_deref(), Some("new visual"));
        assert_eq!(update.motion_prompt, Some(None));
        assert_eq!(update.negative_prompt, Some(None));
        assert_eq!(update.duration_secs, Some(6));
        assert_eq!(
            update.downstream_strategy,
            DownstreamConflictStrategy::KeepDownstream
        );
        assert!(!update.is_empty());

        for value in [
            json!({"visual_prompt": null, "downstream_strategy": "reject"}),
            json!({"duration_secs": null, "downstream_strategy": "reject"}),
            json!({"visual_prompt": "x"}),
            json!({"visual_prompt": "x", "downstream_strategy": "legacy"}),
            json!({"visual_prompt": "x", "downstream_strategy": "reject", "unknown": true}),
        ] {
            assert!(serde_json::from_value::<UpdateChapterVideoPromptInput>(value).is_err());
        }

        let empty: UpdateChapterVideoPromptInput = serde_json::from_value(json!({
            "downstream_strategy": "reject"
        }))
        .expect("wire contract leaves business empty-check to application boundary");
        assert!(empty.is_empty());

        assert!(
            serde_json::from_value::<OptimizeChapterVideoPromptInput>(json!({
                "instruction": "tighten composition"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<DeleteChapterVideoPromptInput>(json!({
                "prompt_id": 1,
                "downstream_strategy": "delete_downstream"
            }))
            .is_ok()
        );
    }

    #[test]
    fn chapter_video_prompt_preview_override_has_one_strict_contract() {
        let missing = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({}))
            .expect("missing fields remain distinguishable from explicit null");
        assert!(missing.is_empty());
        assert_eq!(missing.motion_prompt, None);
        assert_eq!(missing.reference_image_ids_json, None);

        let cleared = serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(json!({
            "motion_prompt": null,
            "negative_prompt": null,
            "reference_image_ids_json": null
        }))
        .expect("nullable override fields support explicit clearing");
        assert!(!cleared.is_empty());
        assert_eq!(cleared.motion_prompt, Some(None));
        assert_eq!(cleared.negative_prompt, Some(None));
        assert_eq!(cleared.reference_image_ids_json, Some(None));

        for value in [
            json!({"visual_prompt": null}),
            json!({"duration_secs": null}),
            json!({"unknown": true}),
        ] {
            assert!(
                serde_json::from_value::<PreviewChapterVideoPromptOverrideInput>(value).is_err()
            );
        }
    }
}
