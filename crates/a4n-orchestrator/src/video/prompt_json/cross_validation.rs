use std::collections::{HashMap, HashSet};

use a4n_core::{error::AppError, prompt_language::PromptOutputLanguage};

use super::contracts::{VideoPromptShot, VideoSegmentPlan, VideoShootingScriptProjection};
use super::validation::{validate_model_shot, validate_shot};

pub(in crate::video) fn validate_segment_plan_against_shooting_script_projection(
    segment_plan: &[VideoSegmentPlan],
    shooting_script_projection: &VideoShootingScriptProjection,
) -> Result<(), AppError> {
    if segment_plan.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划不能为空".to_string(),
        ));
    }
    let shot_by_id = shooting_script_projection
        .shots()
        .map(|shot| (shot.source_shot_id.as_str(), shot))
        .collect::<HashMap<_, _>>();
    let shot_order = shooting_script_projection
        .shots()
        .enumerate()
        .map(|(index, shot)| (shot.source_shot_id.as_str(), index))
        .collect::<HashMap<_, _>>();
    let mut covered_by_shot = HashMap::<String, HashMap<String, i32>>::new();
    let mut segment_count_by_shot = HashMap::<String, i32>::new();

    for segment in segment_plan {
        let shot = shot_by_id
            .get(segment.source_shot_id.as_str())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频分段规划 #{} 的 source_shot_id {} 不存在",
                    segment.segment_number, segment.source_shot_id
                ))
            })?;
        if segment.segment_index_in_shot < 1 || segment.shot_segment_count < 1 {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的镜头内编号和数量必须大于 0",
                segment.segment_number
            )));
        }
        *segment_count_by_shot
            .entry(segment.source_shot_id.clone())
            .or_default() += 1;
        let allowed_beat_ids = shot
            .action_beats
            .iter()
            .map(|beat| beat.beat_id.as_str())
            .collect::<HashSet<_>>();
        let mut seen_in_segment = HashSet::new();
        for beat_id in &segment.covered_beat_ids {
            if beat_id.trim().is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 不能包含空 ID",
                    segment.segment_number
                )));
            }
            if !allowed_beat_ids.contains(beat_id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 包含不属于镜头 {} 的 beat_id：{}",
                    segment.segment_number, segment.source_shot_id, beat_id
                )));
            }
            if !seen_in_segment.insert(beat_id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 包含重复 beat_id：{}",
                    segment.segment_number, beat_id
                )));
            }
            *covered_by_shot
                .entry(segment.source_shot_id.clone())
                .or_default()
                .entry(beat_id.clone())
                .or_default() += 1;
        }
    }

    validate_segment_plan_shot_order(segment_plan, &shot_order)?;
    validate_segment_plan_shot_counts(segment_plan, &segment_count_by_shot)?;
    for shot in shooting_script_projection.shots() {
        let actual_segment_count = segment_count_by_shot
            .get(&shot.source_shot_id)
            .copied()
            .unwrap_or_default();
        if actual_segment_count < shot.minimum_readable_segment_count {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划镜头 {} 至少需要 {} 个可读片段，实际只有 {} 个",
                shot.source_shot_id, shot.minimum_readable_segment_count, actual_segment_count
            )));
        }
        let counts = covered_by_shot.get(&shot.source_shot_id);
        for beat in &shot.action_beats {
            let count = counts
                .and_then(|items| items.get(&beat.beat_id))
                .copied()
                .unwrap_or(0);
            if beat.required && count != 1 {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划必须且只能覆盖镜头 {} 的必需动作节拍 beat_id {} 一次，实际覆盖 {} 次",
                    shot.source_shot_id, beat.beat_id, count
                )));
            }
            if count > 1 {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划重复覆盖镜头 {} 的动作节拍 beat_id {}，实际覆盖 {} 次",
                    shot.source_shot_id, beat.beat_id, count
                )));
            }
        }
        validate_segment_characters(segment_plan, shot)?;
    }
    Ok(())
}

fn validate_segment_characters(
    segment_plan: &[VideoSegmentPlan],
    shot: &a4n_core::video_execution::VideoExecutionShotV2,
) -> Result<(), AppError> {
    for segment in segment_plan
        .iter()
        .filter(|segment| segment.source_shot_id == shot.source_shot_id)
    {
        for goal in &segment.emotion_action_goals {
            ensure_participant(
                &goal.character,
                &shot.participants,
                segment.segment_number,
                "emotion_action_goals.character",
            )?;
        }
        for action in &segment.performance_plan.action_script {
            ensure_participant(
                &action.character,
                &shot.participants,
                segment.segment_number,
                "performance_plan.action_script.character",
            )?;
        }
    }
    Ok(())
}

fn ensure_participant(
    character: &str,
    participants: &[String],
    segment_number: i32,
    field: &str,
) -> Result<(), AppError> {
    if participants
        .iter()
        .any(|participant| participant == character)
    {
        return Ok(());
    }
    Err(AppError::InvalidArgument(format!(
        "视频分段规划 #{} 的 {}={} 必须属于所在场景 participants",
        segment_number, field, character
    )))
}

fn validate_segment_plan_shot_order(
    segment_plan: &[VideoSegmentPlan],
    shot_order: &HashMap<&str, usize>,
) -> Result<(), AppError> {
    let mut last_order = None;
    let mut closed_shots = HashSet::new();
    let mut active_shot_id: Option<&str> = None;
    for segment in segment_plan {
        let source_shot_id = segment.source_shot_id.as_str();
        let order = shot_order.get(source_shot_id).copied().ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 source_shot_id {} 不存在",
                segment.segment_number, segment.source_shot_id
            ))
        })?;
        if last_order.is_some_and(|last| order < last) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 source_shot_id {} 不能回到前一个镜头",
                segment.segment_number, segment.source_shot_id
            )));
        }
        if active_shot_id != Some(source_shot_id) {
            if let Some(previous) = active_shot_id {
                closed_shots.insert(previous);
            }
            if closed_shots.contains(source_shot_id) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} 的 source_shot_id {} 被非连续拆散",
                    segment.segment_number, segment.source_shot_id
                )));
            }
            active_shot_id = Some(source_shot_id);
        }
        last_order = Some(order);
    }
    Ok(())
}

fn validate_segment_plan_shot_counts(
    segment_plan: &[VideoSegmentPlan],
    segment_count_by_shot: &HashMap<String, i32>,
) -> Result<(), AppError> {
    let mut next_index_by_shot = HashMap::<String, i32>::new();
    for segment in segment_plan {
        let actual_count = segment_count_by_shot
            .get(&segment.source_shot_id)
            .copied()
            .unwrap_or(0);
        if segment.shot_segment_count != actual_count {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 shot_segment_count 必须等于镜头 {} 实际 segment 数量 {}，实际为 {}",
                segment.segment_number,
                segment.source_shot_id,
                actual_count,
                segment.shot_segment_count
            )));
        }
        let expected_index = next_index_by_shot
            .entry(segment.source_shot_id.clone())
            .and_modify(|index| *index += 1)
            .or_insert(1);
        if segment.segment_index_in_shot != *expected_index {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 segment_index_in_shot 必须是镜头 {} 内连续编号 {}，实际为 {}",
                segment.segment_number,
                segment.source_shot_id,
                *expected_index,
                segment.segment_index_in_shot
            )));
        }
    }
    Ok(())
}

pub(in crate::video) fn validate_video_prompt_shot_against_segment(
    shot: VideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot(shot, &segment.source_span_ids, prompt_output_language)
}

pub(in crate::video) fn validate_model_video_prompt_shot_against_segment(
    shot: VideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_model_shot(shot, &segment.source_span_ids, prompt_output_language)
}
