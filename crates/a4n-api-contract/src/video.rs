use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use a4n_provider_contract::VideoProviderConfig;
use a4n_video_contract::{VideoProviderPollTarget, VideoSegmentPlanV2};

pub use a4n_domain::{AssemblyPostprocessOperation, CanvasFrameKind, DownstreamConflictStrategy};
pub use a4n_video_contract::VideoSegmentExecutionRecipeV1;

use crate::serde_update::{deserialize_non_null_update_field, deserialize_nullable_update_field};

use super::generation_tasks::GenerationStartRequestMeta;

/// Host-facing lookup boundary for persisted video provider configuration.
///
/// Runtime may apply a local snapshot or user-scope policy before falling back
/// to this owner; the implementation remains free of runtime state and ORM
/// types.
#[async_trait::async_trait]
pub trait VideoProviderConfigPort: Send + Sync {
    async fn by_id(&self, config_id: i64) -> Result<VideoProviderConfig, a4n_error::AppError>;

    async fn default(&self) -> Result<VideoProviderConfig, a4n_error::AppError>;

    async fn for_provider_job(
        &self,
        project_id: Option<i64>,
        provider_kind: &str,
        provider_job_id: &str,
    ) -> Result<VideoProviderConfig, a4n_error::AppError>;
}

/// Host-facing video owner boundary for provider-job recovery.
///
/// The video owner keeps permission checks and status transitions; Runtime
/// supplies only the provider execution adapter from `a4n-video-contract`.
#[async_trait::async_trait]
pub trait VideoProviderExecutionApplicationPort: Send + Sync {
    async fn poll_target(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<VideoProviderPollTarget, a4n_error::AppError>;

    async fn poll_provider_job(
        &self,
        user_id: i64,
        video_id: i64,
        config: Option<&VideoProviderConfig>,
        host_port: &dyn a4n_video_contract::VideoProviderHostPort,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Host-provided cleanup for media that becomes unreachable after a prompt
/// mutation. The video owner decides which IDs are safe to remove.
#[async_trait::async_trait]
pub trait VideoPromptDownstreamCleanupPort: Send + Sync {
    async fn delete_downstream_media(
        &self,
        image_ids: &[i64],
        video_ids: &[i64],
    ) -> Result<(), a4n_error::AppError>;
}

/// Input shared by chapter video request preview and generation.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateChapterVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub prompt_id: i64,
    #[serde(default)]
    pub config_id: Option<i64>,
}

/// Input shared by reference-media video request preview and generation.
#[derive(Debug, Clone, Deserialize)]
pub struct GenerateReferenceMediaVideoInput {
    pub user_id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub input: ReferenceMediaVideoInput,
}

/// Host-facing request preview boundary for all video entry points.
///
/// Video validation and request construction stay in the video owner. Runtime
/// supplies only the provider and media preview adapters.
#[async_trait::async_trait]
pub trait VideoRequestPreviewApplicationPort: Send + Sync {
    async fn preview_canvas(
        &self,
        input: crate::canvas::GenerateCanvasVideoInput,
        host_port: &dyn a4n_video_contract::CanvasVideoPreviewHostPort,
    ) -> Result<a4n_video_contract::CanvasVideoRequestPreview, a4n_error::AppError>;

    async fn preview_chapter(
        &self,
        host_port: &dyn a4n_video_contract::ChapterVideoPreviewHostPort,
        input: GenerateChapterVideoInput,
        prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
    ) -> Result<a4n_video_contract::ChapterVideoRequestPreview, a4n_error::AppError>;

    async fn preview_reference_media(
        &self,
        host_port: &dyn a4n_video_contract::ReferenceMediaVideoPreviewHostPort,
        input: GenerateReferenceMediaVideoInput,
    ) -> Result<a4n_video_contract::ReferenceMediaVideoRequestPreview, a4n_error::AppError>;
}

/// Host-facing boundary for the video generation worker.
///
/// Runtime owns scheduling and supplies provider/prompt adapters. Reservation,
/// validation, checkpointing, and result commits remain in the video owner.
#[async_trait::async_trait]
pub trait VideoGenerationApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &StartVideoGenerationInput,
    ) -> Result<
        crate::generation_tasks::GenerationTaskStart,
        crate::generation_tasks::GenerationTaskStartError,
    >;

    async fn run_task(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: crate::generation_tasks::GenerationTaskHandle,
        host_port: &dyn a4n_video_contract::VideoGenerationHostPort,
    ) -> Result<(), String>;

    async fn run_prompt_task(
        &self,
        user_id: i64,
        input: StartVideoGenerationInput,
        handle: crate::generation_tasks::GenerationTaskHandle,
        host_port: &dyn a4n_video_contract::VideoPromptGenerationHostPort,
    ) -> Result<(), String>;
}

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

/// Host-facing read boundary for persisted chapter video prompts.
///
/// Chapter ownership, authorization and ORM projection stay in the video
/// owner; Runtime only routes stable summaries.
#[async_trait::async_trait]
pub trait VideoPromptReadApplicationPort: Send + Sync {
    async fn list_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<ChapterVideoPromptSummary>, a4n_error::AppError>;

    async fn validate_project_chapter_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing read/write boundary for the chapter video timeline.
///
/// Timeline revision and ordering rules stay in the video owner; Runtime only
/// routes the stable timeline contract.
#[async_trait::async_trait]
pub trait VideoPromptTimelineApplicationPort: Send + Sync {
    async fn get_video_timeline(
        &self,
        user_id: i64,
        input: GetVideoTimelineInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;

    async fn commit_inserted_segment_draft(
        &self,
        user_id: i64,
        input: CommitInsertedSegmentDraftInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;

    async fn move_video_timeline_item(
        &self,
        user_id: i64,
        input: MoveInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;

    async fn delete_video_timeline_item(
        &self,
        user_id: i64,
        input: DeleteInsertedSegmentInput,
    ) -> Result<VideoTimelineSummary, a4n_error::AppError>;
}

/// Host-facing boundary for loading and updating a segment-plan draft.
///
/// Workflow/checkpoint validation and the summary projection stay in the video
/// owner; Runtime only routes the contract.
#[async_trait::async_trait]
pub trait VideoPromptDraftApplicationPort: Send + Sync {
    async fn load_video_segment_plan_draft(
        &self,
        user_id: i64,
        input: LoadVideoSegmentPlanDraftInput,
    ) -> Result<VideoSegmentPlanDraftSummary, a4n_error::AppError>;

    async fn update_video_segment_plan_draft_recipe(
        &self,
        user_id: i64,
        input: UpdateVideoSegmentPlanDraftRecipeInput,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing mutation boundary for chapter video prompts and segment recipes.
///
/// Video ownership keeps authorization, dependency planning, persistence and
/// projection. Runtime supplies only cleanup and prompt rendering capabilities.
#[async_trait::async_trait]
pub trait VideoPromptMutationApplicationPort: Send + Sync {
    async fn update_chapter_video_segment_recipe(
        &self,
        user_id: i64,
        prompt_id: i64,
        recipe: VideoSegmentExecutionRecipeV1,
    ) -> Result<Vec<VideoSegmentExecutionRecipeV1>, a4n_error::AppError>;

    async fn update_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: UpdateChapterVideoPromptInput,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, a4n_error::AppError>;

    async fn delete_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        downstream_strategy: DownstreamConflictStrategy,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<(), a4n_error::AppError>;

    async fn optimize_chapter_video_prompt(
        &self,
        user_id: i64,
        prompt_id: i64,
        input: OptimizeChapterVideoPromptInput,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
        cleanup: &dyn VideoPromptDownstreamCleanupPort,
    ) -> Result<ChapterVideoPromptSummary, a4n_error::AppError>;
}

/// Host-facing read boundary for persisted video records and validated media.
#[async_trait::async_trait]
pub trait VideoRecordReadApplicationPort: Send + Sync {
    async fn playback_source(
        &self,
        user_id: i64,
        video_id: i64,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn list_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<GeneratedVideoSummary>, a4n_error::AppError>;

    async fn validated_reference_images(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        reference_image_ids_json: Option<&Value>,
    ) -> Result<Vec<crate::image::GeneratedImageSummary>, a4n_error::AppError>;
}

/// Host-facing mutation boundary for ordinary video-record commands.
#[async_trait::async_trait]
pub trait VideoRecordMutationApplicationPort: Send + Sync {
    async fn update(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateGeneratedVideoInput,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn select_segment(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        video_id: i64,
        input: SelectSegmentVideoInput,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn delete(&self, user_id: i64, video_id: i64) -> Result<(), a4n_error::AppError>;
}

/// Host-facing video import and trim boundary.
///
/// Runtime supplies inspected file facts; the video owner creates records and
/// enforces project, chapter, and workflow invariants.
#[async_trait::async_trait]
pub trait VideoImportApplicationPort: Send + Sync {
    async fn create_imported_chapter_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        prompt_id: i64,
        properties: a4n_video_contract::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;

    async fn create_imported_reference_media_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        properties: a4n_video_contract::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
        segment_plan_prompt_id: i64,
        target_source_shot_id: &str,
        target_segment_index: i32,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;

    async fn create_imported_assembly_insert_video_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        insert_after_video_id: Option<i64>,
        properties: a4n_video_contract::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;

    async fn create_imported_canvas_video_record(
        &self,
        project_id: i64,
        properties: a4n_video_contract::ImportedVideoProperties,
        source_file_name: &str,
        source_extension: &str,
        imported_at: &str,
    ) -> Result<VideoImportRecordSummary, a4n_error::AppError>;

    async fn prepare_trim(
        &self,
        user_id: i64,
        video_id: i64,
        start_secs: f32,
        end_secs: f32,
    ) -> Result<VideoTrimPlan, a4n_error::AppError>;

    async fn create_trimmed_record(
        &self,
        plan: VideoTrimPlan,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Host-facing Assembly planning and push boundary.
#[async_trait::async_trait]
pub trait VideoAssemblyApplicationPort: Send + Sync {
    async fn assembly_records(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        operation: &str,
    ) -> Result<AssemblyVideoRecords, a4n_error::AppError>;

    async fn prepare_assembly_postprocess_edit(
        &self,
        push_video_id: i64,
        operation: AssemblyPostprocessOperation,
    ) -> Result<AssemblyPostprocessPlan, a4n_error::AppError>;

    async fn prepare_assembly_postprocess_source(
        &self,
        plan: &AssemblyPostprocessPlan,
        source_video_id: Option<i64>,
        has_uploaded_source: bool,
    ) -> Result<AssemblyPostprocessSourcePlan, a4n_error::AppError>;

    async fn create_assembly_postprocess_edit_record(
        &self,
        plan: AssemblyPostprocessPlan,
        input: AssemblyPostprocessEditRecordInput,
    ) -> Result<PreparedAssemblyPostprocessEditRecord, a4n_error::AppError>;

    async fn create_assembly_push_record(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
        clips: Vec<AssemblyPushClipInput>,
        end_video_id: Option<i64>,
    ) -> Result<PreparedAssemblyPushRecord, a4n_error::AppError>;

    async fn update_assembly_push_status(
        &self,
        user_id: i64,
        video_id: i64,
        input: UpdateAssemblyPushStatusInput,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;
}

/// Host-facing completion and media-cleanup lifecycle boundary.
#[async_trait::async_trait]
pub trait VideoRecordLifecycleApplicationPort: Send + Sync {
    async fn mark_error_status_and_prompt(
        &self,
        video_id: i64,
        prompt_id: Option<i64>,
        status: &str,
        message: &str,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn create_video_tail_frame(
        &self,
        input: VideoTailFrameInput,
    ) -> Result<VideoTailFrameSummary, a4n_error::AppError>;

    async fn delete_video_tail_frame_assets(
        &self,
        video_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn complete(
        &self,
        input: CompleteVideoRecordInput,
    ) -> Result<VideoRecordCompletion, VideoRecordCompletionError>;

    async fn delete_record_with_cleanup(&self, video_id: i64) -> Result<(), a4n_error::AppError>;
}

/// Host-facing canvas-media preparation and persistence boundary.
#[async_trait::async_trait]
pub trait VideoCanvasMediaApplicationPort: Send + Sync {
    async fn prepare_canvas_assembly(
        &self,
        project_id: i64,
        video_ids: Vec<i64>,
        title: Option<&str>,
    ) -> Result<CanvasAssemblyPlan, a4n_error::AppError>;

    async fn create_canvas_assembly_record(
        &self,
        project_id: i64,
        plan: CanvasAssemblyPlan,
    ) -> Result<GeneratedVideoSummary, a4n_error::AppError>;

    async fn prepare_canvas_frame(
        &self,
        project_id: i64,
        video_id: i64,
        frame_kind: CanvasFrameKind,
        at_secs: Option<f32>,
    ) -> Result<CanvasFramePlan, a4n_error::AppError>;

    async fn create_canvas_frame_record(
        &self,
        plan: CanvasFramePlan,
        file_path: String,
        file_size: i64,
    ) -> Result<crate::image::GeneratedImageSummary, a4n_error::AppError>;
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
    pub timeline_item_id: Option<i64>,
    pub is_favorite: bool,
    pub created_at: String,
    pub updated_at: String,
}

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
    pub file_size: i64,
    pub prompt_id: Option<i64>,
    pub metadata_json: Option<Value>,
    pub duration_secs: f32,
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
    DurationWriteback {
        error: a4n_error::AppError,
    },
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
