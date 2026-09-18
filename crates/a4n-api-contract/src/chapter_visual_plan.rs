use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use a4n_domain::ImageSize;
pub use a4n_domain::chapter_visual_plan::*;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanSourceInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SaveChapterVisualPlanDraftInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub record_id: Option<i64>,
    pub expected_revision: Option<i32>,
    /// Supply the source hash used for a source-dependent rewrite.
    #[serde(default)]
    pub expected_source_hash: Option<String>,
    /// Required for an Agent write bound to an editor view; obtained by inspect_editor_target.
    #[serde(default)]
    pub editor_revision: Option<String>,
    pub plan: ChapterVisualPlanV3,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanRecordInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChapterVisualPlanRecordSummary {
    pub id: i64,
    pub plan_version: i32,
    pub revision: i32,
    pub status: ChapterVisualPlanStatus,
    pub authoring_mode: ChapterVisualPlanAuthoringMode,
    pub source_content_sha256: String,
    pub plan_sha256: String,
    pub confirmed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanView {
    pub record: ChapterVisualPlanRecordSummary,
    pub plan: ChapterVisualPlanV3,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanSetView {
    pub source: ChapterVisualPlanSourceView,
    pub starter_plan: ChapterVisualPlanV3,
    pub draft: Option<ChapterVisualPlanView>,
    pub confirmed: Option<ChapterVisualPlanView>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChapterVisualPlanSourceView {
    pub chapter_number: i32,
    pub title: Option<String>,
    pub content: String,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ConfirmedChapterVisualPlanView {
    pub visual_plan: ChapterVisualPlanView,
    pub invalidated_project_ids: Vec<i64>,
}

/// Read only the requested saved-plan projection. Pin record/revision when
/// continuing a paged read; source offsets count Unicode characters.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReadChapterVisualPlanInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
    #[serde(default)]
    pub record_id: Option<i64>,
    #[serde(default)]
    pub expected_revision: Option<i32>,
    pub view: VisualPlanReadView,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum VisualPlanReadView {
    Outline {
        #[serde(default)]
        offset: u32,
        #[serde(default)]
        limit: Option<u32>,
    },
    Point {
        point_id: String,
    },
    Segment {
        segment_id: String,
    },
    Source {
        #[serde(default)]
        offset: u32,
        #[serde(default)]
        limit: Option<u32>,
        #[serde(default)]
        expected_hash: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VisualPlanOutlineItem {
    pub segment_id: String,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub point_count: usize,
    pub scene_binding: Option<SceneBindingV3>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VisualPlanProjection {
    Outline {
        items: Vec<VisualPlanOutlineItem>,
        total: usize,
        next_offset: Option<u32>,
    },
    Point {
        point: VisualPointV3,
    },
    Segment {
        segment: VisualSegmentV3,
        points: Vec<VisualPointV3>,
    },
    Source {
        text: String,
        offset: u32,
        total_chars: usize,
        next_offset: Option<u32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanReadResult {
    pub resource: Option<crate::agent::AgentResourceRef>,
    pub source_hash: String,
    pub projection: VisualPlanProjection,
}

pub use a4n_domain::chapter_visual_plan::VisualPlanEditOperationV3 as VisualPlanEditOperation;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EditChapterVisualPlanDraftInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub record_id: i64,
    pub expected_revision: i32,
    #[serde(default)]
    pub expected_source_hash: Option<String>,
    /// Required for editor-bound Agent calls; obtained with inspect_editor_target.
    #[serde(default)]
    pub editor_revision: Option<String>,
    pub operation: VisualPlanEditOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VisualPlanFieldChange {
    pub object_id: String,
    pub field: String,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum VisualPlanValidationMode {
    Draft,
    Confirmed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ValidateChapterVisualPlanInput {
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub record_id: i64,
    pub expected_revision: i32,
    pub mode: VisualPlanValidationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct VisualPlanDiagnostic {
    pub path: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VisualPlanValidationResult {
    pub mode: VisualPlanValidationMode,
    pub valid: bool,
    /// The existing domain validator returns the first failure, not all errors.
    pub diagnostics: Vec<VisualPlanDiagnostic>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanEditResult {
    pub visual_plan: ChapterVisualPlanView,
    pub resource: crate::agent::AgentResourceRef,
    pub changed: bool,
    pub changes: Vec<VisualPlanFieldChange>,
    pub validation: VisualPlanValidationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterVisualPlanValidationResult {
    pub resource: crate::agent::AgentResourceRef,
    pub validation: VisualPlanValidationResult,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct VisualPlanPointGenerationOptions {
    pub source_chapter_id: i64,
    pub record_id: i64,
    pub expected_revision: i32,
    pub point_id: String,
    pub user_description: String,
    pub config_id: Option<i64>,
    #[serde(default)]
    pub size: Option<ImageSize>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PreviewVisualPlanPointInput {
    pub project_id: i64,
    pub options: VisualPlanPointGenerationOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct VisualPlanPointGenerationPreview {
    pub prompt: String,
    pub reference_image_ids: Vec<i64>,
    pub aspect_ratio: String,
    pub plan_sha256: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct StartVisualPlanPromptGenerationInput {
    pub meta: crate::generation_tasks::GenerationStartRequestMeta,
    pub project_id: i64,
    pub source_chapter_id: i64,
    pub record_id: i64,
    pub expected_revision: i32,
    pub segment_id: String,
    pub user_description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualPlanPromptCandidate {
    pub record_id: i64,
    pub source_chapter_id: i64,
    pub plan_revision: i32,
    pub plan_sha256: String,
    pub segment_id: String,
    pub prompt: String,
    pub user_description: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadVisualPlanPromptCandidateInput {
    pub project_id: i64,
    pub generation_run_id: i64,
}

/// Host-facing read boundary for chapter visual plans.
///
/// Authorization, persistence reads and API projection stay with the visual
/// plan owner; Runtime only routes complete results.
#[async_trait::async_trait]
pub trait ChapterVisualPlanReadApplicationPort: Send + Sync {
    async fn preview_point_generation(
        &self,
        user_id: i64,
        input: PreviewVisualPlanPointInput,
    ) -> Result<VisualPlanPointGenerationPreview, a4n_error::AppError>;

    async fn prompt_candidate(
        &self,
        user_id: i64,
        input: ReadVisualPlanPromptCandidateInput,
    ) -> Result<VisualPlanPromptCandidate, a4n_error::AppError>;
    fn project_editor(
        &self,
        plan: &ChapterVisualPlanV3,
        view: &VisualPlanReadView,
    ) -> Result<VisualPlanProjection, a4n_error::AppError>;

    async fn get(
        &self,
        user_id: i64,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<ChapterVisualPlanSetView, a4n_error::AppError>;

    async fn read(
        &self,
        user_id: i64,
        input: ReadChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanReadResult, a4n_error::AppError>;

    async fn validate(
        &self,
        user_id: i64,
        input: ValidateChapterVisualPlanInput,
    ) -> Result<ChapterVisualPlanValidationResult, a4n_error::AppError>;
}

/// Host-facing mutation boundary for chapter visual plans.
///
/// Validation, revision checks, persistence and invalidation stay with the
/// visual plan owner; Runtime only routes mutation results.
#[async_trait::async_trait]
pub trait ChapterVisualPlanMutationApplicationPort: Send + Sync {
    async fn start_prompt_generation(
        &self,
        user_id: i64,
        input: &StartVisualPlanPromptGenerationInput,
    ) -> Result<
        crate::generation_tasks::GenerationTaskStart,
        crate::generation_tasks::GenerationTaskStartError,
    >;

    async fn run_prompt_generation(
        &self,
        user_id: i64,
        input: StartVisualPlanPromptGenerationInput,
        handle: crate::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String>;
    async fn save_draft(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<ChapterVisualPlanView, a4n_error::AppError>;

    async fn edit_draft(
        &self,
        user_id: i64,
        input: EditChapterVisualPlanDraftInput,
    ) -> Result<ChapterVisualPlanEditResult, a4n_error::AppError>;

    async fn confirm(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<ConfirmedChapterVisualPlanView, a4n_error::AppError>;

    async fn discard(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<(), a4n_error::AppError>;
}

/// Host-facing persistence boundary for ordinary visual-plan records.
///
/// Collaborative local editing remains on the richer mutation port until its
/// projection and validation contracts are extracted without duplication.
#[async_trait::async_trait]
pub trait ChapterVisualPlanRecordPersistenceApplicationPort: Send + Sync {
    async fn save_draft(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<ChapterVisualPlanView, a4n_error::AppError>;

    async fn confirm(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<ConfirmedChapterVisualPlanView, a4n_error::AppError>;

    async fn discard(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<(), a4n_error::AppError>;
}
