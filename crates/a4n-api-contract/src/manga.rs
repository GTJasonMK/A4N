use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::DownstreamConflictStrategy;
pub use a4n_domain::manga::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V2, MANGA_PAGE_PLAN_V3, MangaCameraAngleV2,
    MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2,
    MangaPageGenerationSchemaVersion, MangaPageImportanceV2, MangaPagePacingV2,
    MangaPagePlanSchemaVersion, MangaPagePlanV3, MangaPageRoleV2, MangaPageV3,
    MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2, MangaPipelineStartStage,
    MangaReadingFlowV2, MangaReferenceAssetKind, MangaShootingBeatRefV3, MangaShotScaleV2,
    MangaTextLayerKindV2, MangaTextLayerV2, MangaTextPlacementV2,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPipelineStartInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_number: i32,
    pub start_from_stage: Option<MangaPipelineStartStage>,
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
pub struct BindMangaReferenceImageInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    #[schemars(length(min = 1))]
    pub slot_id: String,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaReferenceTargetType {
    Character,
    Visual,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MangaReferenceBindingStatus {
    Missing,
    Bound,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaBoundReferenceAsset {
    pub asset_kind: MangaReferenceAssetKind,
    pub asset_id: i64,
    pub label: String,
    pub image_path: String,
    pub prompt: String,
    pub portrait_id: Option<i64>,
    pub profile_id: Option<i64>,
    pub portrait_type: Option<String>,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub is_primary: Option<bool>,
    pub image_id: Option<i64>,
    pub image_type: Option<String>,
    pub chapter_id: Option<i64>,
    pub panel_id: Option<String>,
    pub is_favorite: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaReferenceBindingSummary {
    pub slot_id: String,
    pub target_type: MangaReferenceTargetType,
    pub target_name: Option<String>,
    pub label: String,
    pub purpose: String,
    pub required: bool,
    pub asset_kind: MangaReferenceAssetKind,
    pub status: MangaReferenceBindingStatus,
    pub bound_asset: Option<MangaBoundReferenceAsset>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MangaPromptSourceSummary {
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub source_revision_key: String,
    pub content_sha256: String,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub shooting_script_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MangaPromptSummary {
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,
    pub page_number: i32,
    pub panel_number: i32,
    pub prompt_type: String,
    pub image_prompt: String,
    pub dialogue_text: Option<String>,
    pub shooting_script_id: i64,
    pub shooting_script_revision: i32,
    pub page_plan: MangaPageV3,
    pub drawing_prompt: MangaPageDrawingPromptV2,
    pub reference_bindings: Vec<MangaReferenceBindingSummary>,
    pub source: MangaPromptSourceSummary,
    pub generated_image_id: Option<i64>,
    pub status: String,
    pub created_at: String,
    pub updated_at: String,
}
