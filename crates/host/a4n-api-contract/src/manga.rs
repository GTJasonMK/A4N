use crate::generation_tasks::GenerationStartRequestMeta;
use crate::generation_tasks::{
    GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio_util::sync::CancellationToken;

pub use a4n_domain::manga::{
    MANGA_PAGE_DRAWING_PROMPT_V2, MANGA_PAGE_GENERATION_V4, MANGA_PAGE_PIPELINE_STATE_V1,
    MANGA_PAGE_PLAN_V4, MANGA_PAGE_SLOT_LIMIT, MANGA_PAGE_SLOTS_V1, MangaBoundReferenceAsset,
    MangaCameraAngleV2, MangaChapterComicSynthesisV1, MangaPageAdaptationV1, MangaPageContentV1,
    MangaPageDrawingPromptSchemaVersion, MangaPageDrawingPromptV2, MangaPageGenerationLayoutV4,
    MangaPageGenerationSchemaVersion, MangaPageImportanceV2, MangaPagePacingV2,
    MangaPagePipelineErrorV1, MangaPagePipelineStage, MangaPagePipelineStateV1,
    MangaPagePipelineStatus, MangaPagePlanSchemaVersion, MangaPagePlanV4,
    MangaPagePromptSourceSummary, MangaPageReferenceDiscoveryV1, MangaPageReferenceImageV1,
    MangaPageReferenceNeedV1, MangaPageReferenceTargetKind, MangaPageRoleV2, MangaPageSlotV1,
    MangaPageSlotsSchemaVersion, MangaPageSlotsV1, MangaPageV4, MangaPanelFunctionV2,
    MangaPanelSizeV2, MangaPanelV2, MangaPipelineArtifactRecord, MangaPipelineArtifactStage,
    MangaPreprocessingV3, MangaReadingFlowV2, MangaReferenceAssetKind, MangaReferenceBindingAction,
    MangaReferenceBindingSummary, MangaShotScaleV2, MangaTextLayerKindV2, MangaTextLayerV2,
    MangaTextPlacementV2,
};
pub use a4n_domain::media_authoring::AdaptationBeatRef;

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

#[async_trait::async_trait]
pub trait MangaPipelineArtifactApplicationPort: Send + Sync {
    async fn list_artifacts(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPipelineArtifactRecord>, a4n_error::AppError>;

    async fn save_artifact(
        &self,
        user_id: i64,
        input: SaveMangaPipelineArtifactInput,
    ) -> Result<MangaPipelineArtifactRecord, a4n_error::AppError>;

    async fn preview_artifact_deletion(
        &self,
        user_id: i64,
        input: PreviewMangaPipelineArtifactDeletionInput,
    ) -> Result<MangaPipelineArtifactDeletionPreview, a4n_error::AppError>;

    async fn delete_artifact(
        &self,
        user_id: i64,
        input: DeleteMangaPipelineArtifactInput,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing Manga prompt mutation boundary.
///
/// The Manga owner keeps authorization, persistence and prompt projection.
#[async_trait::async_trait]
pub trait MangaRecordMutationApplicationPort: Send + Sync {
    async fn delete_page(
        &self,
        user_id: i64,
        input: DeleteMangaPageInput,
    ) -> Result<(), a4n_error::AppError>;

    async fn bind_reference_image(
        &self,
        user_id: i64,
        input: BindMangaReferenceImageInput,
    ) -> Result<MangaPagePromptSummary, a4n_error::AppError>;

    async fn update_prompt(
        &self,
        user_id: i64,
        input: UpdateMangaPromptInput,
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
    ) -> Result<(), a4n_error::AppError>;

    async fn optimize_prompt(
        &self,
        user_id: i64,
        input: OptimizeMangaPromptInput,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
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
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub page_number: Option<u32>,
    pub stop_after: Option<MangaPipelineStopAfter>,
    #[serde(default)]
    pub run_stage: Option<MangaPipelineArtifactStage>,
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
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub page_number: Option<u32>,
    pub stop_after: Option<MangaPipelineStopAfter>,
    #[serde(default)]
    pub run_stage: Option<MangaPipelineArtifactStage>,
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
    PageAdaptation,
    Storyboard,
    ReferenceDiscovery,
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
pub struct DeleteMangaPageInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    #[schemars(range(min = 1))]
    pub page_number: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaveMangaPipelineArtifactInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    pub stage: MangaPipelineArtifactStage,
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub page_number: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(length(min = 1))]
    pub expected_prompt_updated_at: Option<String>,
    pub payload: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteMangaPipelineArtifactInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    pub stage: MangaPipelineArtifactStage,
    /// Omit to delete this stage across all pages in the chapter, including dependent outputs.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub page_number: Option<u32>,
    #[schemars(length(min = 64, max = 64))]
    pub expected_impact_fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct PreviewMangaPipelineArtifactDeletionInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub chapter_id: i64,
    pub stage: MangaPipelineArtifactStage,
    /// Omit to preview this stage across all pages in the chapter.
    #[serde(default)]
    #[schemars(range(min = 1))]
    pub page_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MangaPipelineArtifactDeletionItem {
    pub artifact_id: Option<i64>,
    pub stage: MangaPipelineArtifactStage,
    pub page_number: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MangaPipelineArtifactDeletionPreview {
    pub target: MangaPipelineArtifactDeletionItem,
    /// Concrete affected artifacts; includes target-stage page records for a chapter-wide target.
    pub downstream: Vec<MangaPipelineArtifactDeletionItem>,
    pub fingerprint: String,
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
pub struct PrepareStandaloneComicSourceInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub asset_id: i64,
    #[schemars(range(min = 1))]
    pub version: i32,
    #[schemars(range(min = 0))]
    pub start_section: i32,
    #[schemars(range(min = 0))]
    pub end_section: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct MangaStandaloneSourceSummary {
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub content_sha256: String,
    pub asset_id: i64,
    pub version: i32,
    pub start_section: i32,
    pub end_section: i32,
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
    async fn prepare_standalone_source(
        &self,
        user_id: i64,
        input: PrepareStandaloneComicSourceInput,
    ) -> Result<MangaStandaloneSourceSummary, a4n_error::AppError>;

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
        page_number: Option<u32>,
        stop_after: Option<MangaPipelineStopAfter>,
        run_stage: Option<MangaPipelineArtifactStage>,
        prompt_renderer: &dyn crate::prompt::PromptRenderPort,
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
        page_number: Option<u32>,
        resume_from_checkpoint: bool,
        stop_after: Option<MangaPipelineStopAfter>,
        run_stage: Option<MangaPipelineArtifactStage>,
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
    pub action: MangaReferenceBindingAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateMangaPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    #[schemars(length(min = 1))]
    pub image_prompt: String,
    #[schemars(length(min = 1))]
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct OptimizeMangaPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
    #[schemars(length(min = 1))]
    pub instruction: String,
    #[schemars(length(min = 1))]
    pub expected_updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteMangaPromptInput {
    #[schemars(range(min = 1))]
    pub prompt_id: i64,
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
    pub reference_discovery: Option<MangaPageReferenceDiscoveryV1>,
    pub drawing_prompt: Option<MangaPageDrawingPromptV2>,
    pub stale_stages: Vec<MangaPipelineArtifactStage>,
    pub orphaned: bool,
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

#[cfg(test)]
mod tests {
    use super::{OptimizeMangaPromptInput, UpdateMangaPromptInput};
    use serde_json::json;

    #[test]
    fn manga_prompt_mutations_require_the_displayed_version() {
        let expected_updated_at = "2026-10-15 00:00:00";
        assert!(
            serde_json::from_value::<UpdateMangaPromptInput>(json!({
                "prompt_id": 1,
                "image_prompt": "新提示词",
                "expected_updated_at": expected_updated_at
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<UpdateMangaPromptInput>(json!({
                "prompt_id": 1,
                "image_prompt": "新提示词"
            }))
            .is_err()
        );
        assert!(
            serde_json::from_value::<OptimizeMangaPromptInput>(json!({
                "prompt_id": 1,
                "instruction": "增加景深",
                "expected_updated_at": expected_updated_at
            }))
            .is_ok()
        );
        assert!(
            serde_json::from_value::<OptimizeMangaPromptInput>(json!({
                "prompt_id": 1,
                "instruction": "增加景深"
            }))
            .is_err()
        );
    }
}
