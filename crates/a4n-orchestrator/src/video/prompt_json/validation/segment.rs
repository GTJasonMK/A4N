mod content;
mod performance;

use std::collections::HashSet;

use a4n_error::AppError;
use a4n_video_contract::VIDEO_SEGMENT_PLAN_V2;

use super::super::VideoSegmentPlan;
use super::{required_text, validate_duration};
use content::{
    validate_segment_emotion_action_goals, validate_segment_emotional_arc,
    validate_segment_material_coverage, validate_segment_spatial_anchors,
    validate_segment_video_intent,
};
use performance::validate_segment_performance_plan;

pub(in crate::video::prompt_json) fn validate_segment_plan(
    mut segment: VideoSegmentPlan,
) -> Result<VideoSegmentPlan, AppError> {
    if segment.schema_version != VIDEO_SEGMENT_PLAN_V2 {
        return Err(AppError::InvalidArgument(format!(
            "视频分段规划 schema_version 必须是 {VIDEO_SEGMENT_PLAN_V2}"
        )));
    }
    if segment.segment_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频分段规划 segment_number 必须大于 0".to_string(),
        ));
    }
    segment.title = required_text(segment.title, "视频分段规划 title")?;
    segment.scene_summary = required_text(segment.scene_summary, "视频分段规划 scene_summary")?;
    segment.blocking_summary =
        required_text(segment.blocking_summary, "视频分段规划 blocking_summary")?;
    validate_segment_spatial_anchors(&segment.spatial_anchors)?;
    validate_segment_emotion_action_goals(&segment.emotion_action_goals)?;
    validate_segment_video_intent(&segment.video_intent)?;
    validate_segment_emotional_arc(&segment.emotional_arc)?;
    validate_duration(segment.duration_secs, "视频分段规划 duration_secs")?;
    validate_segment_performance_plan(&segment.performance_plan, segment.duration_secs)?;
    validate_segment_material_coverage(&segment.material_coverage)?;
    if segment.source_span_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划 source_span_ids 不能为空".to_string(),
        ));
    }
    segment.aspect_ratio = required_text(segment.aspect_ratio, "视频分段规划 aspect_ratio")?;
    validate_covered_beat_ids(&segment.covered_beat_ids)?;
    Ok(segment)
}

fn validate_covered_beat_ids(ids: &[String]) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for id in ids {
        if id.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "视频分段规划 covered_beat_ids 不能包含空 beat_id".to_string(),
            ));
        }
        if !seen.insert(id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 covered_beat_ids 包含重复 beat_id：{id}"
            )));
        }
    }
    Ok(())
}
