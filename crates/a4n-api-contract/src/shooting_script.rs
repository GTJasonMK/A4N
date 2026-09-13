use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::shooting_script::{
    ChapterShootingScriptSchemaVersion, ChapterShootingScriptV1, ShootingBlockingV1,
    ShootingCameraAngleV1, ShootingCameraHeightV1, ShootingCameraMovementV1,
    ShootingContinuityIntentV1, ShootingCoverageV1, ShootingDialogueCueV1,
    ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingPerformanceBeatV1,
    ShootingSceneV1, ShootingShotSizeV1, ShootingShotV1, ShootingSoundDesignV1,
    ShootingSpatialContinuityV1, ShootingTransitionContractV1, ShootingTransitionTypeV1,
    ShootingViewpointModeV1, ShootingViewpointV1, ShootingVisualConstraintsV3,
    ShootingVisualPlanSourceV1,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterShootingScriptSourceInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    pub visual_plan_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterShootingScriptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub visual_plan_id: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EditChapterShootingScriptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub expected_revision: i32,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct ChapterShootingScriptRecordSummary {
    pub id: i64,
    pub revision: i32,
    pub visual_plan_id: Option<i64>,
    pub visual_plan_version: Option<i32>,
    pub visual_plan_sha256: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterShootingScriptView {
    pub record: ChapterShootingScriptRecordSummary,
    pub script: ChapterShootingScriptV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterShootingScriptUnitProgressView {
    pub workflow_run_id: i64,
    pub script: ChapterShootingScriptV1,
    pub completed_unit_count: usize,
    pub total_unit_count: usize,
    pub completed_record: Option<ChapterShootingScriptRecordSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EditedChapterShootingScriptView {
    pub shooting_script: ChapterShootingScriptView,
    pub invalidated_project_ids: Vec<i64>,
}

/// Host-facing boundary for persisted shooting-script records.
///
/// Source resolution, visual-plan consistency and persistence stay with the
/// shooting-script owner; hosts only route typed reads and edits.
#[async_trait::async_trait]
pub trait ShootingScriptRecordApplicationPort: Send + Sync {
    async fn current(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptView>, a4n_error::AppError>;

    async fn unit_progress(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptUnitProgressView>, a4n_error::AppError>;

    async fn edit(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScriptView, a4n_error::AppError>;
}
