use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub use a4n_domain::shooting_script::{
    ChapterShootingScriptRecordSummary, ChapterShootingScriptSchemaVersion,
    ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptV1, EditChapterShootingScriptProgressInput,
    EditedChapterShootingScriptProgressView, ShootingBlockingV1, ShootingCameraAngleV1,
    ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingContinuityIntentV1,
    ShootingCoverageV1, ShootingDialogueCueV1, ShootingDirectingStatementV1, ShootingEditingModeV1,
    ShootingPerformanceBeatV1, ShootingSceneV1, ShootingScriptPlanSyncV1, ShootingShotSizeV1,
    ShootingShotV1, ShootingSoundDesignV1, ShootingSpatialContinuityV1,
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterShootingScriptInput {
    #[schemars(range(min = 1))]
    pub project_id: i64,
    #[schemars(range(min = 1))]
    pub source_chapter_id: i64,
    #[schemars(range(min = 1))]
    pub visual_plan_id: i64,
    /// 模型辅助局部更新：只替换这些镜头，其他镜头和场景内容原样保留。
    /// 为空时重新生成整章。
    #[serde(default)]
    pub shot_ids: Vec<String>,
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
    #[serde(default)]
    pub shot_ids: Vec<String>,
    pub mode: ShootingScriptGenerationMode,
}

impl StartChapterShootingScriptGenerationInput {
    pub fn generation_input(&self) -> GenerateChapterShootingScriptInput {
        GenerateChapterShootingScriptInput {
            project_id: self.project_id,
            source_chapter_id: self.source_chapter_id,
            visual_plan_id: self.visual_plan_id,
            shot_ids: self.shot_ids.clone(),
        }
    }
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
    pub script: ChapterShootingScriptV1,
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
    pub script: ChapterShootingScriptV1,
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
    pub script: ChapterShootingScriptV1,
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
