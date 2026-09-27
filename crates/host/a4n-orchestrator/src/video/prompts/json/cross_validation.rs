use std::collections::{HashMap, HashSet};

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_error::AppError;

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
        .enumerate()
        .map(|(index, shot)| (shot.source_shot_id.as_str(), (index, shot)))
        .collect::<HashMap<_, _>>();
    let mut seen_shots = HashSet::new();
    let mut last_shot_order = None;

    for segment in segment_plan {
        let &(order, shot) = shot_by_id
            .get(segment.source_shot_id.as_str())
            .ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "视频分段规划 #{} 的 source_shot_id {} 不存在",
                    segment.segment_number, segment.source_shot_id
                ))
            })?;
        if !seen_shots.insert(segment.source_shot_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "导演镜头 {} 必须对应一个片段，不能重新拆镜头",
                segment.source_shot_id
            )));
        }
        if last_shot_order.is_some_and(|last| order < last) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 的 source_shot_id {} 不能回到前一个镜头",
                segment.segment_number, segment.source_shot_id
            )));
        }
        last_shot_order = Some(order);
        let director = &shot.director_constraints;
        if segment.director_constraints.as_ref() != Some(director)
            || segment.duration_secs
                != director
                    .video_duration_secs()
                    .map_err(AppError::InvalidArgument)?
            || segment.aspect_ratio
                != director
                    .frame
                    .aspect_ratio()
                    .map_err(AppError::InvalidArgument)?
            || segment.segment_index_in_shot != 1
            || segment.shot_segment_count != 1
        {
            return Err(AppError::InvalidArgument(format!(
                "镜头 {} 的视频规划改变了导演编排约束",
                shot.source_shot_id
            )));
        }
        let allowed_beat_ids = shot
            .action_beats
            .iter()
            .enumerate()
            .map(|(index, beat)| (beat.beat_id.as_str(), index))
            .collect::<HashMap<_, _>>();
        let mut seen_in_segment = HashSet::new();
        let mut last_beat_order = None;
        for beat_id in &segment.covered_beat_ids {
            if beat_id.trim().is_empty() {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 不能包含空 ID",
                    segment.segment_number
                )));
            }
            let Some(&order) = allowed_beat_ids.get(beat_id.as_str()) else {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 包含不属于镜头 {} 的 beat_id：{}",
                    segment.segment_number, segment.source_shot_id, beat_id
                )));
            };
            if !seen_in_segment.insert(beat_id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} covered_beat_ids 包含重复 beat_id：{}",
                    segment.segment_number, beat_id
                )));
            }
            if last_beat_order.is_some_and(|last| order < last) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划 #{} 的 covered_beat_ids 必须遵循拍摄剧本节拍顺序",
                    segment.segment_number
                )));
            }
            last_beat_order = Some(order);
        }
        for beat in &shot.action_beats {
            if beat.required && !seen_in_segment.contains(beat.beat_id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "视频分段规划必须覆盖镜头 {} 的必需动作节拍 beat_id {} 一次",
                    shot.source_shot_id, beat.beat_id
                )));
            }
        }
        validate_segment_characters(segment, shot)?;
    }
    for shot in shooting_script_projection.shots() {
        if !seen_shots.contains(shot.source_shot_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划缺少导演镜头 {} 的片段",
                shot.source_shot_id
            )));
        }
    }
    Ok(())
}

fn validate_segment_characters(
    segment: &VideoSegmentPlan,
    shot: &a4n_video_contract::VideoExecutionShotV3,
) -> Result<(), AppError> {
    if shot.participants.is_empty() {
        if !segment.emotion_action_goals.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 对应纯环境镜头，emotion_action_goals 必须为空，不能把环境或场景名伪装成人物",
                segment.segment_number
            )));
        }
        if !segment.performance_plan.action_script.is_empty() {
            return Err(AppError::InvalidArgument(format!(
                "视频分段规划 #{} 对应纯环境镜头，performance_plan.action_script 必须为空，环境动态应写入 time_blocks",
                segment.segment_number
            )));
        }
        return Ok(());
    }
    if segment.emotion_action_goals.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "视频分段规划 #{} emotion_action_goals 不能为空",
            segment.segment_number
        )));
    }
    if segment.performance_plan.action_script.is_empty() {
        return Err(AppError::InvalidArgument(format!(
            "视频分段规划 #{} performance_plan.action_script 不能为空",
            segment.segment_number
        )));
    }
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

pub(in crate::video) fn validate_video_prompt_shot_against_segment(
    shot: VideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_shot(shot, segment, prompt_output_language)
}

pub(in crate::video) fn validate_model_video_prompt_shot_against_segment(
    shot: VideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoPromptShot, AppError> {
    validate_model_shot(shot, segment, prompt_output_language)
}
