use std::collections::HashSet;

use a4n_error::AppError;
use serde_json::Value;

use a4n_domain::source_spans::SourceSpan;
use a4n_video_contract::VIDEO_SEGMENT_PLAN_V2;

use super::contracts::{
    VideoSegmentPlan, VideoSegmentPlanInput, VideoSegmentPlanResponse,
    VideoShootingScriptProjection,
};
use super::{
    derive_segment_source_span_ids, parse_strict_json_response, validate_segment_plan,
    validate_source_span_ids,
};

pub(in crate::video) fn video_segment_plan_input_values(
    segments: &[VideoSegmentPlan],
    context: &str,
) -> Result<Vec<Value>, AppError> {
    segments
        .iter()
        .map(VideoSegmentPlanInput::from)
        .map(|segment| {
            serde_json::to_value(segment).map_err(|error| {
                AppError::InvalidArgument(format!("{context} segments 条目序列化失败: {error}"))
            })
        })
        .collect()
}

pub(in crate::video) fn validate_stored_video_segment_plan(
    segments: Vec<VideoSegmentPlan>,
    source_spans: &[SourceSpan],
    expected_aspect_ratio: &str,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    segments.into_iter().map(|segment| {
        let segment_number = segment.segment_number;
        let aspect_ratio = match &segment.director_constraints {
            Some(director) => director.frame.aspect_ratio().map_err(AppError::InvalidArgument)?,
            None => expected_aspect_ratio.to_string(),
        };
        if segment.aspect_ratio != aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} aspect_ratio 必须等于 {aspect_ratio}，实际为 {}",
                segment.aspect_ratio
            )));
        }
        if !segment.reference_image_ids.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "已存视频分段规划 segment_number {segment_number} 包含已废弃的 reference_image_ids；片段规划不再绑定具体素材，请删除旧规划并重新生成"
            )));
        }
        let source_span_ids =
            if segment.source_span_ids.is_empty() && segment.director_constraints.is_some() {
                Vec::new()
            } else {
                validate_source_span_ids(
                    segment.source_span_ids.clone(),
                    source_spans,
                    &format!("已存视频分段规划 segment_number {}", segment_number),
                )?
            };
        let segment = VideoSegmentPlan {
            source_span_ids,
            ..segment
        };
        validate_segment_plan(segment)
    }).collect()
}

pub(in crate::video) fn parse_video_segment_plan_response(
    raw: &str,
    source_spans: &[SourceSpan],
    projection: &VideoShootingScriptProjection,
) -> Result<Vec<VideoSegmentPlan>, AppError> {
    let response: VideoSegmentPlanResponse = parse_strict_json_response(raw, "视频分段规划")?;
    if response.segments.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划响应必须包含至少一个 segments 条目".to_string(),
        ));
    }
    i32::try_from(response.segments.len()).map_err(|_| {
        AppError::InvalidArgument("视频分段规划 segments 数量超出 i32 范围".to_string())
    })?;
    let mut seen_shots = HashSet::new();
    let mut segments = Vec::with_capacity(response.segments.len());
    for (index, segment) in response.segments.into_iter().enumerate() {
        let segment_number = i32::try_from(index + 1).map_err(|_| {
            AppError::InvalidArgument("视频分段规划 segment_number 超出 i32 范围".to_string())
        })?;
        if !seen_shots.insert(segment.source_shot_id.clone()) {
            return Err(AppError::InvalidArgument(format!(
                "导演镜头 {} 必须对应一个片段，不能重新拆镜头",
                segment.source_shot_id
            )));
        }
        let source_span_ids = derive_segment_source_span_ids(
            projection,
            &segment.source_shot_id,
            &segment.covered_beat_ids,
            segment_number,
        )?;
        let shot = projection
            .shot(&segment.source_shot_id)
            .ok_or_else(|| AppError::InvalidArgument("视频片段引用了不存在的镜头".into()))?;
        let source_span_ids = if source_span_ids.is_empty() {
            source_span_ids
        } else {
            validate_source_span_ids(
                source_span_ids,
                source_spans,
                &format!("视频分段规划 segment_number {segment_number}"),
            )?
        };
        let segment = VideoSegmentPlan {
            director_constraints: Some(shot.director_constraints.clone()),
            schema_version: VIDEO_SEGMENT_PLAN_V2.to_string(),
            segment_number,
            source_shot_id: segment.source_shot_id,
            segment_index_in_shot: 1,
            shot_segment_count: 1,
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
            source_span_ids,
            duration_secs: shot
                .director_constraints
                .video_duration_secs()
                .map_err(AppError::InvalidArgument)?,
            aspect_ratio: shot
                .director_constraints
                .frame
                .aspect_ratio()
                .map_err(AppError::InvalidArgument)?,
            reference_image_ids: Vec::new(),
        };
        let segment = validate_segment_plan(segment)?;
        segments.push(segment);
    }
    Ok(segments)
}
