use std::collections::HashSet;

use a4n_core::{
    error::AppError,
    video_execution::{
        VIDEO_EXECUTION_UNIT_V2, VIDEO_SEGMENT_PLAN_V2, VideoActionBridgeUnitV1,
        VideoContentUnitV2, VideoEmotionActionGoalV1, VideoEmotionalArcV1,
        VideoExecutionUnitKindV2, VideoExecutionUnitV2, VideoIntentV1, VideoMaterialCoverageV1,
        VideoPerformanceActionV1, VideoPerformancePlanV2, VideoPerformanceTimeBlockV1,
        VideoSegmentPlanV2, VideoSpatialAnchorV1,
    },
};

use super::VideoSegmentPlan;

pub(in crate::video) type VideoUnitPlan = VideoExecutionUnitV2;

pub(in crate::video) fn expected_boundary_frame_count(segment_plan: &[VideoSegmentPlan]) -> usize {
    let unit_count = expected_video_unit_count(segment_plan);
    if unit_count == 0 { 0 } else { unit_count + 1 }
}

pub(in crate::video) fn expected_video_unit_count(segment_plan: &[VideoSegmentPlan]) -> usize {
    segment_plan.len()
        + segment_plan
            .iter()
            .take(segment_plan.len().saturating_sub(1))
            .filter(|segment| segment_requires_action_bridge(segment))
            .count()
}

pub(in crate::video) fn video_units_for_segment_plan(
    segment_plan: &[VideoSegmentPlan],
    duration_secs: i32,
) -> Result<Vec<VideoUnitPlan>, AppError> {
    if segment_plan
        .iter()
        .any(|segment| segment.duration_secs != duration_secs)
    {
        return Err(AppError::InvalidArgument(
            "视频片段规划时长与当前视频单元时长不一致".to_string(),
        ));
    }
    let mut units = Vec::with_capacity(expected_video_unit_count(segment_plan));
    for (index, segment) in segment_plan.iter().enumerate() {
        push_content_unit(&mut units, segment, duration_secs)?;
        let Some(next_segment) = segment_plan.get(index + 1) else {
            continue;
        };
        if segment_requires_action_bridge(segment) {
            push_action_bridge_unit(&mut units, segment, next_segment, duration_secs)?;
        }
    }
    Ok(units)
}

fn push_content_unit(
    units: &mut Vec<VideoUnitPlan>,
    segment: &VideoSegmentPlan,
    duration_secs: i32,
) -> Result<(), AppError> {
    let unit_number = next_unit_number(units.len())?;
    let mut prompt_segment = segment.clone();
    prompt_segment.segment_number = unit_number;
    prompt_segment.duration_secs = duration_secs;
    prompt_segment.transition_to_next = None;
    units.push(VideoExecutionUnitV2 {
        schema_version: VIDEO_EXECUTION_UNIT_V2.to_string(),
        unit_number,
        duration_secs,
        start_frame_number: Some(unit_number),
        end_frame_number: unit_number.checked_add(1),
        storyboard_number: Some(unit_number),
        kind: VideoExecutionUnitKindV2::Content(VideoContentUnitV2 {
            source_segment_number: segment.segment_number,
        }),
        prompt_segment,
    });
    Ok(())
}

fn push_action_bridge_unit(
    units: &mut Vec<VideoUnitPlan>,
    from: &VideoSegmentPlan,
    to: &VideoSegmentPlan,
    duration_secs: i32,
) -> Result<(), AppError> {
    let transition = from.transition_to_next.as_ref().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "片段 {} 声明动作桥但缺少 transition_to_next",
            from.segment_number
        ))
    })?;
    let preserved_action = transition
        .preserved_action
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("动作桥 preserved_action 不能为空".to_string()))?;
    let unit_number = next_unit_number(units.len())?;
    let actor = from
        .performance_plan
        .action_script
        .first()
        .map(|action| action.character.clone())
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "片段 {} performance_plan.action_script 为空，无法创建动作桥",
                from.segment_number
            ))
        })?;
    let source_span_ids =
        unique_strings(from.source_span_ids.iter().chain(to.source_span_ids.iter()));
    let spatial_anchors =
        unique_anchors(from.spatial_anchors.iter().chain(to.spatial_anchors.iter()));
    let prompt_segment = VideoSegmentPlanV2 {
        schema_version: VIDEO_SEGMENT_PLAN_V2.to_string(),
        segment_number: unit_number,
        source_shot_id: format!("bridge:{}->{}", from.source_shot_id, to.source_shot_id),
        segment_index_in_shot: 1,
        shot_segment_count: 1,
        covered_beat_ids: Vec::new(),
        title: format!("动作桥：{} → {}", from.title, to.title),
        scene_summary: format!(
            "只表现动作接点“{}”，把“{}”的尾态交给“{}”的首态，不新增剧情。",
            preserved_action, from.title, to.title
        ),
        blocking_summary: format!(
            "从“{}”的结尾站位开始，以“{}”完成可见交接，停在“{}”的开场站位。",
            from.blocking_summary, preserved_action, to.blocking_summary
        ),
        spatial_anchors,
        emotion_action_goals: vec![VideoEmotionActionGoalV1 {
            character: actor.clone(),
            emotion: format!(
                "承接“{}”并进入“{}”",
                from.emotional_arc.end, to.emotional_arc.start
            ),
            visible_action: preserved_action.to_string(),
        }],
        video_intent: VideoIntentV1 {
            hook: preserved_action.to_string(),
            audience_effect: "让动作、空间和时间交接可读，避免无依据跳变".to_string(),
            rhythm: "单一连续动作桥".to_string(),
            visual_priority: preserved_action.to_string(),
        },
        emotional_arc: VideoEmotionalArcV1 {
            start: from.emotional_arc.end.clone(),
            turn: preserved_action.to_string(),
            end: to.emotional_arc.start.clone(),
            function: "显式动作接点".to_string(),
        },
        performance_plan: VideoPerformancePlanV2 {
            movement_path: preserved_action.to_string(),
            body_performance: format!("{actor} 只完成动作桥所需的身体位移和停点"),
            gaze_and_hands: "视线、手部和道具状态只随 preserved_action 连续变化".to_string(),
            pause_or_reaction: "动作完成后留出稳定尾态，不追加新的反应事件".to_string(),
            camera_relationship: "机位只负责看清动作接点与空间交接，不重新设计前后片段主动作"
                .to_string(),
            time_blocks: vec![VideoPerformanceTimeBlockV1 {
                start_sec: 0.0,
                end_sec: f64::from(duration_secs),
                camera: "单一连续镜头或明确动作接点".to_string(),
                subject_action: preserved_action.to_string(),
                continuity_target: to.blocking_summary.clone(),
            }],
            action_script: vec![VideoPerformanceActionV1 {
                character: actor,
                body: "身体姿态只完成动作桥所需变化".to_string(),
                movement: preserved_action.to_string(),
                hands: "手部与道具状态保持动作连续".to_string(),
                gaze: "视线沿动作目标自然交接".to_string(),
                timing: format!("0-{duration_secs} 秒内完成并稳定"),
            }],
        },
        material_coverage: VideoMaterialCoverageV1 {
            required_entities: unique_strings(
                from.material_coverage
                    .required_entities
                    .iter()
                    .chain(to.material_coverage.required_entities.iter()),
            ),
            missing_or_weak_references: unique_strings(
                from.material_coverage
                    .missing_or_weak_references
                    .iter()
                    .chain(to.material_coverage.missing_or_weak_references.iter()),
            ),
            risk_note: "动作桥沿用前后内容单元的实体与连续性约束".to_string(),
        },
        transition_to_next: None,
        source_span_ids,
        duration_secs,
        aspect_ratio: from.aspect_ratio.clone(),
        reference_image_ids: Vec::new(),
    };
    units.push(VideoExecutionUnitV2 {
        schema_version: VIDEO_EXECUTION_UNIT_V2.to_string(),
        unit_number,
        duration_secs,
        start_frame_number: Some(unit_number),
        end_frame_number: unit_number.checked_add(1),
        storyboard_number: Some(unit_number),
        kind: VideoExecutionUnitKindV2::ActionBridge(VideoActionBridgeUnitV1 {
            from_segment_number: from.segment_number,
            to_segment_number: to.segment_number,
            preserved_action: preserved_action.to_string(),
            preserved_sound: transition.preserved_sound.clone(),
            required_cut_point: transition.required_cut_point.clone(),
        }),
        prompt_segment,
    });
    Ok(())
}

fn segment_requires_action_bridge(segment: &VideoSegmentPlan) -> bool {
    segment
        .transition_to_next
        .as_ref()
        .and_then(|transition| transition.preserved_action.as_deref())
        .is_some_and(|action| !action.trim().is_empty())
}

fn next_unit_number(current_len: usize) -> Result<i32, AppError> {
    i32::try_from(current_len + 1)
        .map_err(|_| AppError::InvalidArgument("视频单元编号超出 i32 范围".to_string()))
}

fn unique_strings<'a>(values: impl Iterator<Item = &'a String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .filter(|value| seen.insert((*value).clone()))
        .cloned()
        .collect()
}

fn unique_anchors<'a>(
    values: impl Iterator<Item = &'a VideoSpatialAnchorV1>,
) -> Vec<VideoSpatialAnchorV1> {
    let mut seen = HashSet::new();
    values
        .filter(|value| seen.insert((value.anchor.clone(), value.usage.clone())))
        .cloned()
        .collect()
}

pub(super) fn boundary_frame_expected_segment_numbers(
    segment_plan: &[VideoSegmentPlan],
    frame_number: i32,
) -> Result<Vec<i32>, AppError> {
    let units = units_for_frame_mapping(segment_plan)?;
    if frame_number < 1 || frame_number as usize > units.len() + 1 {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 frame_number {} 超出视频单元范围",
            frame_number
        )));
    }
    let mut numbers = Vec::new();
    if frame_number > 1 {
        extend_unique_numbers(
            &mut numbers,
            &units[(frame_number - 2) as usize].source_segment_numbers(),
        );
    }
    if (frame_number as usize) <= units.len() {
        extend_unique_numbers(
            &mut numbers,
            &units[(frame_number - 1) as usize].source_segment_numbers(),
        );
    }
    Ok(numbers)
}

pub(super) fn boundary_frame_role(
    segment_plan: &[VideoSegmentPlan],
    frame_number: i32,
) -> Result<&'static str, AppError> {
    let count = expected_boundary_frame_count(segment_plan);
    if frame_number < 1 || frame_number as usize > count {
        return Err(AppError::InvalidArgument(format!(
            "边界帧 frame_number {} 超出视频单元范围",
            frame_number
        )));
    }
    if frame_number == 1 {
        Ok("chain_start")
    } else if frame_number as usize == count {
        Ok("chain_end")
    } else {
        Ok("shared_boundary")
    }
}

pub(in crate::video) fn storyboard_expected_segment_numbers(
    segment_plan: &[VideoSegmentPlan],
    storyboard_number: i32,
) -> Result<Vec<i32>, AppError> {
    let units = units_for_frame_mapping(segment_plan)?;
    if storyboard_number < 1 || storyboard_number as usize > units.len() {
        return Err(AppError::InvalidArgument(format!(
            "分镜 frame_number {} 超出视频单元范围",
            storyboard_number
        )));
    }
    Ok(units[(storyboard_number - 1) as usize].source_segment_numbers())
}

pub(super) fn boundary_frame_expected_duration(
    segment_plan: &[VideoSegmentPlan],
    segment_numbers: &[i32],
) -> Result<i32, AppError> {
    let mut duration = None;
    for segment_number in segment_numbers {
        let segment = segment_plan
            .iter()
            .find(|segment| segment.segment_number == *segment_number)
            .ok_or_else(|| {
                AppError::InvalidArgument(format!("边界帧引用了不存在的片段 {}", segment_number))
            })?;
        if duration.is_some_and(|expected| expected != segment.duration_secs) {
            return Err(AppError::InvalidArgument(
                "边界帧关联片段时长不一致".to_string(),
            ));
        }
        duration = Some(segment.duration_secs);
    }
    duration.ok_or_else(|| AppError::InvalidArgument("边界帧 segment_numbers 不能为空".to_string()))
}

fn units_for_frame_mapping(
    segment_plan: &[VideoSegmentPlan],
) -> Result<Vec<VideoUnitPlan>, AppError> {
    let duration_secs = segment_plan
        .first()
        .map(|segment| segment.duration_secs)
        .ok_or_else(|| AppError::InvalidArgument("视频片段规划不能为空".to_string()))?;
    video_units_for_segment_plan(segment_plan, duration_secs)
}

fn extend_unique_numbers(target: &mut Vec<i32>, source: &[i32]) {
    for number in source {
        if !target.contains(number) {
            target.push(*number);
        }
    }
}
