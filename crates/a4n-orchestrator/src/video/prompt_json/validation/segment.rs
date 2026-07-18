mod content;
mod performance;

use std::collections::HashSet;

use a4n_core::error::AppError;
use a4n_core::video_execution::VIDEO_SEGMENT_PLAN_V2;

use super::super::VideoSegmentPlan;
use super::{required_text, validate_duration};
use content::{
    validate_segment_emotion_action_goals, validate_segment_emotional_arc,
    validate_segment_material_coverage, validate_segment_spatial_anchors,
    validate_segment_video_intent,
};
use performance::validate_segment_performance_plan;

pub(in crate::video::prompt_json) fn validate_segment_plan(
    segment: VideoSegmentPlan,
) -> Result<VideoSegmentPlan, AppError> {
    let VideoSegmentPlan {
        schema_version,
        segment_number,
        source_shot_id,
        segment_index_in_shot,
        shot_segment_count,
        covered_beat_ids,
        title,
        scene_summary,
        blocking_summary,
        spatial_anchors,
        emotion_action_goals,
        video_intent,
        emotional_arc,
        performance_plan,
        material_coverage,
        transition_to_next,
        source_span_ids,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
    } = segment;
    if schema_version != VIDEO_SEGMENT_PLAN_V2 {
        return Err(AppError::InvalidArgument(format!(
            "视频分段规划 schema_version 必须是 {VIDEO_SEGMENT_PLAN_V2}"
        )));
    }
    if segment_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频分段规划 segment_number 必须大于 0".to_string(),
        ));
    }
    let title = required_text(title, "视频分段规划 title")?;
    let scene_summary = required_text(scene_summary, "视频分段规划 scene_summary")?;
    let blocking_summary = required_text(blocking_summary, "视频分段规划 blocking_summary")?;
    validate_segment_spatial_anchors(&spatial_anchors)?;
    validate_segment_emotion_action_goals(&emotion_action_goals)?;
    validate_segment_video_intent(&video_intent)?;
    validate_segment_emotional_arc(&emotional_arc)?;
    validate_duration(duration_secs, "视频分段规划 duration_secs")?;
    validate_segment_performance_plan(&performance_plan, duration_secs)?;
    validate_segment_material_coverage(&material_coverage)?;
    if source_span_ids.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划 source_span_ids 不能为空".to_string(),
        ));
    }
    let aspect_ratio = required_text(aspect_ratio, "视频分段规划 aspect_ratio")?;
    validate_covered_beat_ids(&covered_beat_ids)?;
    Ok(VideoSegmentPlan {
        schema_version,
        segment_number,
        source_shot_id,
        segment_index_in_shot,
        shot_segment_count,
        covered_beat_ids,
        title,
        scene_summary,
        blocking_summary,
        spatial_anchors,
        emotion_action_goals,
        video_intent,
        emotional_arc,
        performance_plan,
        material_coverage,
        transition_to_next,
        source_span_ids,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
    })
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
