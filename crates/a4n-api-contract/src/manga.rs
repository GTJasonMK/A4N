use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub use a4n_domain::DownstreamConflictStrategy;
pub use a4n_domain::manga::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V2, MANGA_PAGE_PLAN_V3,
    MangaBoundReferenceAsset, MangaCameraAngleV2, MangaPageDrawingPromptSchemaVersion,
    MangaPageDrawingPromptV2, MangaPageGenerationSchemaVersion, MangaPageImportanceV2,
    MangaPagePacingV2, MangaPagePlanSchemaVersion, MangaPagePlanV3, MangaPagePromptSourceSummary,
    MangaPageRoleV2, MangaPageV3, MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2,
    MangaReadingFlowV2, MangaReferenceAssetKind, MangaReferenceBindingSummary,
    MangaShootingBeatRefV3, MangaShotScaleV2, MangaTextLayerKindV2, MangaTextLayerV2,
    MangaTextPlacementV2,
};

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
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineResumeInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
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
    async fn preflight(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), a4n_error::AppError>;

    async fn run(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_number: i32,
        task_id: String,
        resume_from_checkpoint: bool,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
    ) -> Result<(), a4n_error::AppError>;
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

/// Host-facing Manga pipeline cancellation boundary.
#[async_trait::async_trait]
pub trait MangaPipelineCancellationApplicationPort: Send + Sync {
    async fn cancel(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<i64, a4n_error::AppError>;
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
    pub task_id: Option<String>,
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
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub page_plan: MangaPageV3,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub source: MangaPagePromptSourceSummary,
    pub generated_image_id: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
