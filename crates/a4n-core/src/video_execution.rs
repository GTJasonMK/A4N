//! 结构化拍摄剧本面向视频生成的稳定执行合同。

mod contracts;
mod segment_recipe;
mod timeline;

pub use contracts::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_EXECUTION_PROJECTION_V2, VIDEO_EXECUTION_UNIT_V2,
    VIDEO_SEGMENT_PLAN_V2, VideoActionBridgeUnitV1, VideoActionLoadV1, VideoContentUnitV2,
    VideoDirectingAuditV2, VideoEmotionActionGoalV1, VideoEmotionalArcV1, VideoExecutionBeatV2,
    VideoExecutionProjectionV2, VideoExecutionQualityV2, VideoExecutionRiskLevelV1,
    VideoExecutionRiskV1, VideoExecutionSceneV2, VideoExecutionShotV2, VideoExecutionUnitKindV2,
    VideoExecutionUnitV2, VideoIntentV1, VideoMaterialCoverageV1, VideoPerformanceActionV1,
    VideoPerformancePlanV2, VideoPerformanceTimeBlockV1, VideoSegmentPlanV2,
    VideoSegmentTransitionV1, VideoShootingScriptIdentityV1, VideoSourceSpanRangeV1,
    VideoSpatialAnchorV1,
};
pub use segment_recipe::{
    VIDEO_SEGMENT_EXECUTION_RECIPE_V1, VIDEO_SEGMENT_RECIPES_METADATA_KEY,
    VideoReferenceMediaBindingV1, VideoReferenceMediaRecipeV1, VideoSegmentExecutionRecipeV1,
    VideoSegmentGuidanceModeV1, VideoSegmentWorkflowModeV1, carry_forward_video_segment_recipes,
    default_video_segment_recipes, propagate_adopted_video_change, resolve_video_segment_number,
    update_video_segment_recipe_metadata, validate_video_segment_recipes_for_plan,
    video_segment_content_fingerprint, video_segment_recipe_from_metadata,
    video_segment_recipes_for_plan_from_metadata, video_segment_recipes_from_metadata,
    video_segments_share_recipe_anchor,
};
pub use timeline::{
    NativeVideoTimelineKey, TimelineError, VideoTimelineGap, move_inserted_timeline_item,
    remove_inserted_timeline_item, timeline_gap_index,
};
