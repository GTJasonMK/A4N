use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::shooting_script::{
    ChapterShootingScriptRecordSummary, ChapterShootingScriptSchemaVersion,
    ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptV2, EditChapterShootingScriptProgressInput,
    EditedChapterShootingScriptProgressView, ShootingBlockingV1, ShootingCameraAngleV1,
    ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingContinuityIntentV1,
    ShootingCoverageV1, ShootingDialogueCueV1, ShootingDirectingStatementV1, ShootingEditingModeV1,
    ShootingPerformanceBeatV2, ShootingSceneV2, ShootingScriptPlanSyncV1, ShootingShotSizeV1,
    ShootingShotV2, ShootingSoundDesignV1, ShootingSpatialContinuityV1,
    ShootingTransitionContractV1, ShootingTransitionTypeV1, ShootingViewpointModeV1,
    ShootingViewpointV1, ShootingVisualConstraintsV3, ShootingVisualPlanSourceV1,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ShootingScriptGenerationMode {
    Full,
    NextScene,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartChapterShootingScriptGenerationInput {
    pub meta: crate::generation_tasks::GenerationStartRequestMeta,
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub visual_plan_id: i64,
    /// full 模式下仅更新所选镜头；为空时生成整章，next_scene 不接受此字段的非空值。
    #[serde(default)]
    pub shot_ids: Vec<String>,
    pub mode: ShootingScriptGenerationMode,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct PublishChapterShootingScriptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(length(min = 64, max = 64))]
    pub input_fingerprint_sha256: String,
    #[schemars(range(min = 1))]
    pub visual_plan_id: i64,
    #[schemars(range(min = 1))]
    pub visual_plan_version: i32,
    #[schemars(length(min = 64, max = 64))]
    pub visual_plan_sha256: String,
    pub script: ChapterShootingScriptV2,
    #[schemars(range(min = 1))]
    pub workflow_run_id: i64,
    #[schemars(range(min = 1))]
    pub expected_revision: Option<i32>,
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
    pub script: ChapterShootingScriptV2,
    /// 手动更新：用户已按当前导演编排修订的过时镜头，保存时重新绑定到当前方案约束。
    #[serde(default)]
    pub rebind_shot_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeleteChapterShootingScriptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub record_id: i64,
    #[schemars(range(min = 1))]
    pub expected_revision: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ChapterShootingScriptView {
    pub record: ChapterShootingScriptRecordSummary,
    pub script: ChapterShootingScriptV2,
    /// 相对当前已确认导演编排的逐镜头同步状态。
    pub plan_sync: ShootingScriptPlanSyncV1,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EditedChapterShootingScriptView {
    pub shooting_script: ChapterShootingScriptView,
    pub invalidated_project_ids: Vec<i64>,
}

/// Host-facing read boundary for the persisted current record.
///
/// The unit-progress read remains separate because it returns a durable
/// scene-level generation snapshot rather than a published record.
#[async_trait::async_trait]
pub trait ShootingScriptCurrentReadApplicationPort: Send + Sync {
    async fn current(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptView>, a4n_error::AppError>;
}

/// Host-facing read boundary for generation progress.
///
/// The generation owner persists the snapshot; the planning Worker reads and
/// validates it for Host callers.
#[async_trait::async_trait]
pub trait ShootingScriptUnitProgressReadApplicationPort: Send + Sync {
    async fn unit_progress(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ChapterShootingScriptUnitProgressView>, a4n_error::AppError>;
}

/// Host-facing mutation boundary for persisted shooting-script records.
#[async_trait::async_trait]
pub trait ShootingScriptRecordMutationApplicationPort: Send + Sync {
    async fn edit_progress(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptProgressInput,
    ) -> Result<EditedChapterShootingScriptProgressView, a4n_error::AppError>;

    async fn publish(
        &self,
        user_id: i64,
        input: PublishChapterShootingScriptInput,
    ) -> Result<ChapterShootingScriptView, a4n_error::AppError>;

    async fn edit(
        &self,
        user_id: i64,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScriptView, a4n_error::AppError>;

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteChapterShootingScriptInput,
    ) -> Result<(), a4n_error::AppError>;
}
