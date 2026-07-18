use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;
use serde_json::Value;

use a4n_core::source_spans::SourceSpan;
use a4n_core::video_execution::VIDEO_SEGMENT_PLAN_V2;

use super::contracts::{
    StoredVideoSegmentPlanResponse, VideoSegmentPlan, VideoSegmentPlanDefaults,
    VideoSegmentPlanResponse, VideoShootingScriptProjection,
};
use super::{
    derive_segment_source_span_ids, parse_strict_json_response, validate_continuous_ids,
    validate_segment_plan, validate_source_span_ids,
};

pub(in crate::video) fn parse_video_segment_plan_response(
    raw: &str,
    source_spans: &[SourceSpan],
    shooting_script_projection: &VideoShootingScriptProjection,
    duration_secs: i32,
    default_aspect_ratio: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    parse_video_segment_plan_inner(
        raw,
        source_spans,
        VideoSegmentPlanDefaults {
            shooting_script_projection,
            duration_secs,
            aspect_ratio: default_aspect_ratio,
        },
    )
}

pub(in crate::video) fn parse_video_segment_plan_input_values(
    raw: &str,
    context: &str,
) -> Result<Vec<Value>, AppError> {
    let response: VideoSegmentPlanResponse = parse_strict_json_response(raw, context)?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "{context} 响应必须包含至少一个 segments 条目"
        )));
    }
    response
        .segments
        .into_iter()
        .map(|segment| {
            serde_json::to_value(segment).map_err(|err| {
                AppError::InvalidArgument(format!("{context} segments 条目序列化失败: {err}"))
            })
        })
        .collect()
}

pub(in crate::video) fn parse_stored_video_segment_plan_for_reuse(
    raw: &str,
    source_spans: &[SourceSpan],
    expected_aspect_ratio: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let response: StoredVideoSegmentPlanResponse = serde_json::from_str(raw.trim())?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "已存视频分段规划必须包含至少一个 segments 条目".to_string(),
        ));
    }

    let mut seen_numbers = HashSet::new();
    let mut segments = Vec::with_capacity(response.segments.len());
    for segment in response.segments {
        let segment_number = segment.segment_number;
        if segment.aspect_ratio != expected_aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} aspect_ratio 必须等于当前请求 {expected_aspect_ratio}，实际为 {}；请删除旧规划并重新生成",
                segment.aspect_ratio
            )));
        }
        if !segment.reference_image_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} 包含已废弃的 reference_image_ids；片段规划不再绑定具体素材，请删除旧规划并重新生成"
            )));
        }
        let source_span_ids = validate_source_span_ids(
            segment.source_span_ids.clone(),
            source_spans,
            &format!("已存视频分段规划 segment_number {}", segment_number),
        )?;
        let segment = VideoSegmentPlan {
            source_span_ids,
            ..segment
        };
        let segment = validate_segment_plan(segment)?;
        if !seen_numbers.insert(segment.segment_number) {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {} 重复",
                segment.segment_number
            )));
        }
        segments.push(segment);
    }
    validate_continuous_ids(
        segments.iter().map(|segment| segment.segment_number),
        "已存视频分段规划 segment_number",
    )?;
    Ok(segments)
}

fn parse_video_segment_plan_inner(
    raw: &str,
    source_spans: &[SourceSpan],
    defaults: VideoSegmentPlanDefaults<'_>,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let response: VideoSegmentPlanResponse = serde_json::from_str(raw.trim())?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划响应必须包含至少一个 segments 条目".to_string(),
        ));
    }
    i32::try_from(response.segments.len()).map_err(|_| {
        AppError::InvalidArgument("视频分段规划 segments 数量超出 i32 范围".to_string())
    })?;
    let mut segment_count_by_shot = HashMap::<String, i32>::new();
    for segment in &response.segments {
        let count = segment_count_by_shot
            .entry(segment.source_shot_id.clone())
            .or_default();
        *count = count.checked_add(1).ok_or_else(|| {
            AppError::InvalidArgument("单镜头视频分段数量超出 i32 范围".to_string())
        })?;
    }
    let mut next_index_by_shot = HashMap::<String, i32>::new();
    let mut seen_numbers = HashSet::new();
    let mut segments = Vec::with_capacity(response.segments.len());
    for (index, segment) in response.segments.into_iter().enumerate() {
        let segment_number = i32::try_from(index + 1).map_err(|_| {
            AppError::InvalidArgument("视频分段规划 segment_number 超出 i32 范围".to_string())
        })?;
        let shot_segment_count = segment_count_by_shot
            .get(&segment.source_shot_id)
            .copied()
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频分段规划 segment_number {} 无法推导 shot_segment_count",
                    segment_number
                ))
            })?;
        let derived_index = next_index_by_shot
            .entry(segment.source_shot_id.clone())
            .or_default();
        *derived_index = derived_index.checked_add(1).ok_or_else(|| {
            AppError::InvalidArgument("单镜头 segment_index_in_shot 超出 i32 范围".to_string())
        })?;
        let segment_index_in_shot = *derived_index;
        let source_span_ids = derive_segment_source_span_ids(
            defaults.shooting_script_projection,
            &segment.source_shot_id,
            &segment.covered_beat_ids,
            segment_number,
        )?;
        let segment = VideoSegmentPlan {
            schema_version: VIDEO_SEGMENT_PLAN_V2.to_string(),
            segment_number,
            source_shot_id: segment.source_shot_id,
            segment_index_in_shot,
            shot_segment_count,
            covered_beat_ids: segment.covered_beat_ids,
            title: segment.title,
            scene_summary: segment.scene_summary,
            blocking_summary: segment.blocking_summary,
            spatial_anchors: segment.spatial_anchors,
            emotion_action_goals: segment.emotion_action_goals,
            video_intent: segment.video_intent,
            emotional_arc: segment.emotional_arc,
            performance_plan: segment.performance_plan,
            material_coverage: segment.material_coverage,
            transition_to_next: None,
            source_span_ids: validate_source_span_ids(
                source_span_ids,
                source_spans,
                &format!("视频分段规划 segment_number {}", segment_number),
            )?,
            duration_secs: defaults.duration_secs,
            aspect_ratio: defaults.aspect_ratio.to_string(),
            reference_image_ids: Vec::new(),
        };
        let segment = validate_segment_plan(segment)?;
        if !seen_numbers.insert(segment.segment_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 segment_number {} 重复",
                segment.segment_number
            )));
        }
        segments.push(segment);
    }
    apply_segment_transitions(&mut segments, defaults.shooting_script_projection)?;
    Ok(segments)
}

fn apply_segment_transitions(
    segments: &mut [VideoSegmentPlan],
    projection: &VideoShootingScriptProjection,
) -> Result<(), AppError> {
    use a4n_core::video_execution::VideoSegmentTransitionV1;

    if segments.len() < 2 {
        return Ok(());
    }
    for index in 0..segments.len() - 1 {
        let from_shot_id = segments[index].source_shot_id.clone();
        let to_shot_id = segments[index + 1].source_shot_id.clone();
        if from_shot_id == to_shot_id {
            continue;
        }
        let from_shot = projection.shot(&from_shot_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划引用不存在的 source_shot_id {from_shot_id}"
            ))
        })?;
        let to_shot = projection.shot(&to_shot_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划引用不存在的 source_shot_id {to_shot_id}"
            ))
        })?;
        let Some(contract) = from_shot
            .transition_contracts
            .iter()
            .find(|contract| contract.to_narrative_unit_index == to_shot.narrative_unit_index)
        else {
            continue;
        };
        segments[index].transition_to_next = Some(VideoSegmentTransitionV1 {
            transition_type: contract.transition_type.clone(),
            editing_mode: from_shot.editing_mode,
            preserved_action: contract.preserved_action.clone(),
            preserved_sound: contract.preserved_sound.clone(),
            required_cut_point: contract.required_cut_point.clone(),
        });
    }
    Ok(())
}
