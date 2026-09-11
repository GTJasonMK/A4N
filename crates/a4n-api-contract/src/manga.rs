use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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
