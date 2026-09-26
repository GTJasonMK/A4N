//! 章节结构化拍摄剧本领域合同。
//!
//! 视频、漫画和 Galgame 只能从该合同投影媒介执行计划，不得重新解释章节并维护
//! 第二套场景、镜头、表演节拍或连续性事实。

mod contracts;
mod director_projection;
mod plan_sync;
mod validation;

pub use contracts::{
    CHAPTER_SHOOTING_SCRIPT_V1, ChapterShootingScriptRecordSummary,
    ChapterShootingScriptSchemaVersion, ChapterShootingScriptUnitProgressCheckpoint,
    ChapterShootingScriptUnitProgressView, ChapterShootingScriptV1,
    EditChapterShootingScriptProgressInput, EditStoredChapterShootingScriptInput,
    EditedChapterShootingScriptProgressView, ShootingBlockingV1, ShootingCameraAngleV1,
    ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingContinuityIntentV1,
    ShootingCoverageV1, ShootingDialogueCueV1, ShootingDirectingStatementV1, ShootingEditingModeV1,
    ShootingPerformanceBeatV1, ShootingSceneV1, ShootingShotSizeV1, ShootingShotV1,
    ShootingSoundDesignV1, ShootingSpatialContinuityV1, ShootingTransitionContractV1,
    ShootingTransitionTypeV1, ShootingViewpointModeV1, ShootingViewpointV1,
    ShootingVisualConstraintsV3, ShootingVisualPlanSourceV1,
};
pub use director_projection::{
    DirectorScriptPlan, DirectorScriptUnit, project_visual_plan_for_shooting_script,
    segment_addition_id,
};
pub use plan_sync::ShootingScriptPlanSyncV1;
pub use validation::ShootingScriptValidationError;
