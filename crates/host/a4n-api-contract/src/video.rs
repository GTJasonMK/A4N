use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use a4n_video_contract::VideoSegmentPlanV2;

pub use a4n_domain::video_prompt::ChapterVideoPromptSummary;
pub use a4n_domain::{AssemblyPostprocessOperation, CanvasFrameKind, DownstreamConflictStrategy};
pub use a4n_video_contract::VideoSegmentExecutionRecipeV1;

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

use super::generation_tasks::GenerationStartRequestMeta;

mod assembly;
pub use assembly::*;
mod generation_ports;
pub use generation_ports::*;
mod prompt_ports;
pub use prompt_ports::*;
mod record_ports;
pub use record_ports::*;

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
    /// 根据时间线前后片段和用户提示词生成尚未提交的插入片段草稿。
    PromptInsertedSegment {
        #[schemars(range(min = 1))]
        chapter_id: i64,
        options: VideoInsertedSegmentGenerationOptions,
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
    pub fn kind(&self) -> &'static str {
        self.names().0
    }

    pub fn generation_task_key(&self) -> &'static str {
        self.names().1
    }

    fn names(&self) -> (&'static str, &'static str) {
        match self {
            Self::Chapter { .. } => ("chapter", "video_chapter"),
            Self::ChapterBatch { .. } => ("chapter_batch", "video_chapter_batch"),
            Self::ReferenceMedia { .. } => ("reference_media", "video_reference_media"),
            Self::PromptSegmentPlan { .. } => ("prompt_segment_plan", "video_prompt_segment_plan"),
            Self::PromptInsertedSegment { .. } => {
                ("prompt_inserted_segment", "video_prompt_inserted_segment")
            }
            Self::PromptFrames { .. } => ("prompt_frames", "video_prompt_frames"),
            Self::PromptVideos { .. } => ("prompt_videos", "video_prompt_videos"),
        }
    }

    pub fn is_prompt_generation(&self) -> bool {
        matches!(
            self,
            Self::PromptSegmentPlan { .. }
                | Self::PromptInsertedSegment { .. }
                | Self::PromptFrames { .. }
                | Self::PromptVideos { .. }
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
    pub prompt_output_language: String,
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

/// 插入规划生成输入；只生成草稿，不修改共享原生片段规划。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VideoInsertedSegmentGenerationOptions {
    #[schemars(range(min = 1))]
    pub timeline_id: i64,
    #[schemars(range(min = 1))]
    pub expected_timeline_revision: i64,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub previous_item_id: Option<i64>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub next_item_id: Option<i64>,
    #[schemars(length(min = 1))]
    pub user_prompt: String,
    pub style: String,
    pub prompt_output_language: String,
    #[schemars(range(min = 1, max = 60))]
    pub duration_secs: i32,
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    #[serde(default)]
    pub reference_image_ids: Vec<i64>,
    #[serde(default)]
    pub allow_without_visual_anchor: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoTimelineItemKind {
    Native,
    Inserted,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VideoTimelineItemStatus {
    Active,
    NeedsReanchor,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VideoTimelineItemSummary {
    pub id: i64,
    pub kind: VideoTimelineItemKind,
    pub status: VideoTimelineItemStatus,
    pub display_index: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub segment_number: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_shot_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_prompt_id: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub generated_video_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VideoTimelineSummary {
    pub id: i64,
    pub revision: i64,
    pub items: Vec<VideoTimelineItemSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GetVideoTimelineInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommitInsertedSegmentDraftInput {
    #[schemars(range(min = 1))]
    pub timeline_id: i64,
    #[schemars(range(min = 1))]
    pub expected_timeline_revision: i64,
    #[schemars(range(min = 1))]
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MoveInsertedSegmentInput {
    #[schemars(range(min = 1))]
    pub timeline_id: i64,
    #[schemars(range(min = 1))]
    pub expected_timeline_revision: i64,
    #[schemars(range(min = 1))]
    pub timeline_item_id: i64,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub previous_item_id: Option<i64>,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub next_item_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteInsertedSegmentInput {
    #[schemars(range(min = 1))]
    pub timeline_id: i64,
    #[schemars(range(min = 1))]
    pub expected_timeline_revision: i64,
    #[schemars(range(min = 1))]
    pub timeline_item_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateVideoSegmentPlanDraftRecipeInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    #[schemars(range(min = 1))]
    pub workflow_run_id: i64,
    #[schemars(length(min = 1))]
    pub source_shot_id: String,
    #[schemars(range(min = 1))]
    pub segment_index_in_shot: i32,
    pub recipe: VideoSegmentExecutionRecipeV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LoadVideoSegmentPlanDraftInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    #[schemars(range(min = 1))]
    pub workflow_run_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VideoSegmentPlanDraftSummary {
    pub run: crate::workflow::WorkflowRunSummary,
    pub segments: Vec<VideoSegmentPlanV2>,
    pub recipes: Vec<VideoSegmentExecutionRecipeV1>,
    pub completed_shot_count: usize,
    pub total_shot_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateChapterVideoSegmentPlanInput {
    /// The `updated_at` value returned with the plan being edited.
    #[schemars(length(min = 1))]
    pub expected_updated_at: String,
    #[schemars(length(min = 1))]
    pub segments: Vec<VideoSegmentPlanV2>,
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
    #[schemars(length(min = 1))]
    pub expected_updated_at: String,
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
    #[schemars(length(min = 1))]
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteChapterVideoPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VideoPromptDeletionEffects {
    pub deleted_prompt_ids: Vec<i64>,
    pub deleted_image_ids: Vec<i64>,
    pub deleted_video_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VideoPromptUpdateResult {
    pub prompt: ChapterVideoPromptSummary,
    pub deletion: VideoPromptDeletionEffects,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VideoPromptDeleteResult {
    pub deletion: VideoPromptDeletionEffects,
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
    pub timeline_item_id: Option<i64>,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct VideoRecordDeleteResult {
    pub deleted_video_ids: Vec<i64>,
    pub deleted_prompt_ids: Vec<i64>,
    pub updated_prompts: Vec<ChapterVideoPromptSummary>,
    pub updated_videos: Vec<GeneratedVideoSummary>,
}

#[cfg(test)]
mod tests {
    use super::{
        AssemblyPostprocessOperation, AssemblyPushAction, DeleteChapterVideoPromptInput,
        DownstreamConflictStrategy, OptimizeChapterVideoPromptInput,
        PreviewChapterVideoPromptOverrideInput, UpdateAssemblyPushStatusInput,
        UpdateChapterVideoPromptInput, UpdateGeneratedVideoInput,
    };
    use serde_json::json;

    #[test]
    fn generated_video_update_has_one_strict_patch_contract() {
        let updated_at = "2026-10-15 00:00:00";
        for payload in [
            json!({"is_favorite": true, "expected_updated_at": updated_at}),
            json!({"prompt": "new prompt", "negative_prompt": null, "expected_updated_at": updated_at}),
            json!({"negative_prompt": "blur", "expected_updated_at": updated_at}),
        ] {
            let input: UpdateGeneratedVideoInput = serde_json::from_value(payload.clone()).unwrap();
            let forwarded = serde_json::to_value(&input).unwrap();
            assert_eq!(forwarded, payload);
            assert_eq!(
                serde_json::from_value::<UpdateGeneratedVideoInput>(forwarded).unwrap(),
                input
            );
        }
        let input: UpdateGeneratedVideoInput = serde_json::from_value(json!({
            "prompt": "new prompt",
            "negative_prompt": null,
            "is_favorite": true,
            "expected_updated_at": updated_at
        }))
        .expect("valid patch");
        assert_eq!(input.prompt.as_deref(), Some("new prompt"));
        assert_eq!(input.negative_prompt, Some(None));
        assert_eq!(input.is_favorite, Some(true));
        assert!(!input.is_empty());

        for value in [
            json!({"prompt": null, "expected_updated_at": updated_at}),
            json!({"is_favorite": null, "expected_updated_at": updated_at}),
            json!({"clear_negative_prompt": true, "expected_updated_at": updated_at}),
            json!({"metadata_json": {}, "expected_updated_at": updated_at}),
            json!({"clear_metadata_json": true, "expected_updated_at": updated_at}),
            json!({"insert_after_video_id": 1, "expected_updated_at": updated_at}),
            json!({"move_assembly_insert_to_start": true, "expected_updated_at": updated_at}),
        ] {
            assert!(serde_json::from_value::<UpdateGeneratedVideoInput>(value).is_err());
        }

        let empty: UpdateGeneratedVideoInput = serde_json::from_value(json!({
            "expected_updated_at": updated_at
        }))
        .unwrap();
        assert!(empty.is_empty());
        assert!(serde_json::from_value::<UpdateGeneratedVideoInput>(json!({})).is_err());
    }

    #[test]
    fn assembly_mutations_have_one_strict_action_contract() {
        let status: UpdateAssemblyPushStatusInput =
            serde_json::from_value(json!({"action": "accept"})).unwrap();
        assert_eq!(status.action, AssemblyPushAction::Accept);
        assert!(
            serde_json::from_value::<UpdateAssemblyPushStatusInput>(json!({"action": "unknown"}))
                .is_err()
        );

        let operation: AssemblyPostprocessOperation =
            serde_json::from_value(json!("replace")).unwrap();
        assert_eq!(operation.as_str(), "replace");
        assert!(serde_json::from_value::<AssemblyPostprocessOperation>(json!("crop")).is_err());
    }

    #[test]
    fn chapter_video_prompt_mutations_have_one_strict_contract() {
        let update: UpdateChapterVideoPromptInput = serde_json::from_value(json!({
            "visual_prompt": "new visual",
            "motion_prompt": null,
            "negative_prompt": null,
            "duration_secs": 6,
            "downstream_strategy": "keep_downstream",
            "expected_updated_at": "2026-10-15 00:00:00"
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
            "downstream_strategy": "reject",
            "expected_updated_at": "2026-10-15 00:00:00"
        }))
        .expect("wire contract leaves business empty-check to application boundary");
        assert!(empty.is_empty());

        assert!(
            serde_json::from_value::<UpdateChapterVideoPromptInput>(json!({
                "visual_prompt": "x",
                "downstream_strategy": "reject"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<OptimizeChapterVideoPromptInput>(json!({
                "instruction": "tighten composition",
                "downstream_strategy": "reject",
                "expected_updated_at": "2026-10-15 00:00:00"
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<OptimizeChapterVideoPromptInput>(json!({
                "instruction": "tighten composition",
                "downstream_strategy": "reject"
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
