//! 章节结构化拍摄剧本领域合同。
//!
//! 视频、漫画和 Galgame 只能从该合同投影媒介执行计划，不得重新解释章节并维护
//! 第二套场景、镜头、表演节拍或连续性事实。

pub mod compat;
mod contracts;
mod director_projection;
mod plan_sync;
mod validation;

pub use contracts::{
    CHAPTER_SHOOTING_SCRIPT_V2, ChapterShootingScriptRecordSummary,
    ChapterShootingScriptReferenceRecord, ChapterShootingScriptSchemaVersion,
    ChapterShootingScriptUnitProgressCheckpoint, ChapterShootingScriptUnitProgressView,
    ChapterShootingScriptV2, EditChapterShootingScriptProgressInput,
    EditStoredChapterShootingScriptInput, EditedChapterShootingScriptProgressView,
    ShootingBlockingV1, ShootingCameraAngleV1, ShootingCameraHeightV1, ShootingCameraMovementV1,
    ShootingContinuityIntentV1, ShootingCoverageV1, ShootingDialogueCueV1,
    ShootingDirectingStatementV1, ShootingEditingModeV1, ShootingPerformanceBeatV2,
    ShootingSceneV2, ShootingShotSizeV1, ShootingShotV2, ShootingSoundDesignV1,
    ShootingSpatialContinuityV1, ShootingTransitionContractV1, ShootingTransitionTypeV1,
    ShootingViewpointModeV1, ShootingViewpointV1, ShootingVisualConstraintsV3,
    ShootingVisualPlanSourceV1,
};
pub use director_projection::{
    DirectorScriptPlan, DirectorScriptUnit, project_visual_plan_for_shooting_script,
    segment_addition_id,
};
pub use plan_sync::ShootingScriptPlanSyncV1;
pub use validation::ShootingScriptValidationError;

/// Source identity ignores the local record ID/version of the same director plan.
pub fn source_hash(script: &serde_json::Value) -> Result<String, serde_json::Error> {
    let mut value = script.clone();
    if let Some(source) = value
        .get_mut("visual_plan_source")
        .and_then(serde_json::Value::as_object_mut)
    {
        source.remove("plan_id");
        source.remove("plan_version");
    }
    Ok(crate::source_text_hash(&serde_json::to_string(&value)?))
}
