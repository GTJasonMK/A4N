use crate::generation_tasks::GenerationStartRequestMeta;
use crate::generation_tasks::{
    GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub use a4n_domain::DownstreamConflictStrategy;
pub use a4n_domain::manga::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V4, MANGA_PAGE_PIPELINE_STATE_V1,
    MANGA_PAGE_PLAN_V4, MANGA_PAGE_SLOT_LIMIT, MANGA_PAGE_SLOTS_V1, MangaBoundReferenceAsset,
    MangaCameraAngleV2, MangaPageAdaptationV1, MangaPageContentV1,
    MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2, MangaPageGenerationLayoutV4,
    MangaPageGenerationSchemaVersion, MangaPageImportanceV2, MangaPagePacingV2,
    MangaPagePipelineErrorV1, MangaPagePipelineStage, MangaPagePipelineStateV1,
    MangaPagePipelineStatus, MangaPagePlanSchemaVersion, MangaPagePlanV4,
    MangaPagePromptSourceSummary, MangaPageRoleV2, MangaPageSlotV1, MangaPageSlotsSchemaVersion,
    MangaPageSlotsV1, MangaPageV4, MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2,
    MangaReadingFlowV2, MangaReferenceAssetKind, MangaReferenceBindingSummary, MangaShotScaleV2,
    MangaTextLayerKindV2, MangaTextLayerV2, MangaTextPlacementV2,
};
pub use a4n_domain::media_authoring::AdaptationBeatRef;

/// Host-provided cleanup for media that becomes unreachable after a Manga
/// prompt mutation. The Manga owner decides which image IDs are safe to remove.
#[async_trait::async_trait]
pub trait MangaPromptDownstreamCleanupPort: Send + Sync {
    async fn delete_downstream_images(&self, image_ids: &[i64]) -> Result<(), a4n_error::AppError>;
}

/// Host-facing Manga prompt read boundary.
#[async_trait::async_trait]
pub trait MangaRecordReadApplicationPort: Send + Sync {
    async fn list_page_prompts(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPagePromptSummary>, a4n_error::AppError>;
}

/// Host-facing Manga prompt mutation boundary.
///
/// The Manga owner keeps authorization, persistence and prompt projection;
/// Runtime supplies only downstream cleanup and template rendering ports.
#[async_trait::async_trait]
pub trait MangaRecordMutationApplicationPort: Send + Sync {
    async fn bind_reference_image(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptSummary, a4n_error::AppError>;

    async fn update_prompt(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptSummary, a4n_error::AppError>;

    async fn delete_by_chapter(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<(), a4n_error::AppError>;

    async fn delete_prompt(
        &self,
        user_id: i64,
        input: DeleteMangaPromptInput,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<(), a4n_error::AppError>;

    async fn optimize_prompt(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
        cleanup: &dyn MangaPromptDownstreamCleanupPort,
    ) -> Result<MangaPagePromptSummary, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineStartInput {
    #[serde(default)]
    pub meta: Option<GenerationStartRequestMeta>,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub stop_after: Option<MangaPipelineStopAfter>,
    pub image_size: Option<a4n_domain::ImageSize>,
    #[serde(default)]
    pub resume_from_checkpoint: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineResumeInput {
    #[serde(default)]
    pub meta: Option<GenerationStartRequestMeta>,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub stop_after: Option<MangaPipelineStopAfter>,
    pub image_size: Option<a4n_domain::ImageSize>,
    #[serde(default = "default_resume_from_checkpoint")]
    pub resume_from_checkpoint: bool,
}

fn default_resume_from_checkpoint() -> bool {
    true
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaPipelineStopAfter {
    PageSlots,
    PageContent,
    PageAdaptation,
    Storyboard,
    PagePlanning,
    DrawingPrompt,
    Image,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaChapterInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineStatusesInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(length(min = 1, max = 200))]
    pub chapter_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct MangaPipelinePreviewStep {
    pub key: String,
    pub title: String,
    pub prompt_name: Option<String>,
    pub state: String,
    pub note: Option<String>,
    pub request: Option<Value>,
}

/// Host-facing Manga pipeline execution boundary.
///
/// Authorization and workflow persistence remain with the Manga owner; Runtime
/// supplies rendering and routes the typed results.
#[async_trait::async_trait]
pub trait MangaPipelineExecutionApplicationPort: Send + Sync {
    async fn start_task(
        &self,
        user_id: i64,
        input: &MangaPipelineStartInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_task(
        &self,
        user_id: i64,
        input: MangaPipelineStartInput,
        handle: GenerationTaskHandle,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
        page_image_generator: &dyn MangaPipelinePageImageGenerator,
    ) -> Result<(), String>;

    async fn preflight(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), a4n_error::AppError>;
}

#[async_trait::async_trait]
pub trait MangaPipelinePageImageGenerator: Send + Sync {
    async fn generate_page_image<'a>(
        &self,
        user_id: i64,
        input: crate::image_generation::GeneratePageImageInput,
        workflow_run_id: i64,
        progress: &'a (dyn for<'message> Fn(u32, u32, &'message str) + Send + Sync + 'a),
        cancel_token: CancellationToken,
    ) -> Result<crate::image::GeneratedImageSummary, a4n_error::AppError>;
}

/// Host-facing Manga pipeline request-preview boundary.
#[async_trait::async_trait]
pub trait MangaPipelinePreviewApplicationPort: Send + Sync {
    async fn preview(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        resume_from_checkpoint: bool,
        stop_after: Option<MangaPipelineStopAfter>,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
    ) -> Result<Vec<MangaPipelinePreviewStep>, a4n_error::AppError>;
}

/// Host-facing Manga pipeline status-read boundary.
#[async_trait::async_trait]
pub trait MangaPipelineReadApplicationPort: Send + Sync {
    async fn status(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<MangaPipelineStatusSummary, a4n_error::AppError>;

    async fn statuses(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: Vec<i64>,
    ) -> Result<Vec<MangaPipelineStatusSummary>, a4n_error::AppError>;
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BindMangaReferenceImageInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    #[schemars(length(min = 1))]
    pub character_name: String,
    pub asset_kind: MangaReferenceAssetKind,
    #[schemars(range(min = 1))]
    pub asset_id: i64,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateMangaPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    #[schemars(length(min = 1))]
    pub image_prompt: String,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizeMangaPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    #[schemars(length(min = 1))]
    pub instruction: String,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteMangaPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    pub downstream_strategy: DownstreamConflictStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MangaPipelineStatusSummary {
    pub chapter_id: i64,
    pub pending: i64,
    pub generating: i64,
    pub completed: i64,
    pub failed: i64,
    pub cancelled: i64,
    pub pipeline_status: Option<String>,
    pub stage: Option<String>,
    pub current: Option<i32>,
    pub total: Option<i32>,
    pub message: Option<String>,
    pub task_handle: Option<GenerationTaskHandle>,
    pub can_cancel: bool,
    pub can_resume: bool,
    pub has_checkpoint: bool,
    pub checkpoint_valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MangaPagePromptSummary {
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub page_id: Option<String>,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub page_content: Option<MangaPageContentV1>,
    pub page_adaptation: Option<MangaPageAdaptationV1>,
    pub page_plan: Option<MangaPageV4>,
    pub drawing_prompt: Option<MangaPageDrawingPromptV2>,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub source: MangaPagePromptSourceSummary,
    pub generated_image_id: Option<i64>,
    pub status: String,
    pub pipeline_stage: Option<MangaPagePipelineStage>,
    pub pipeline_status: Option<MangaPagePipelineStatus>,
    pub pipeline_error: Option<MangaPagePipelineErrorV1>,
    pub created_at: String,
    pub updated_at: String,
}
