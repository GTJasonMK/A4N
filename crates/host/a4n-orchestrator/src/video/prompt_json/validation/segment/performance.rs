use a4n_error::AppError;
use a4n_video_contract::{
    VideoPerformanceActionV1, VideoPerformancePlanV2, VideoPerformanceTimeBlockV1,
};

use super::super::required_text;

pub(super) fn validate_segment_performance_plan(
    value: &VideoPerformancePlanV2,
    duration_secs: i32,
) -> Result<(), AppError> {
    for (field, text) in [
        ("movement_path", value.movement_path.clone()),
        ("body_performance", value.body_performance.clone()),
        ("gaze_and_hands", value.gaze_and_hands.clone()),
        ("pause_or_reaction", value.pause_or_reaction.clone()),
        ("camera_relationship", value.camera_relationship.clone()),
    ] {
        required_text(text, &format!("视频分段规划 performance_plan.{field}"))?;
    }
    validate_segment_performance_time_blocks(&value.time_blocks, duration_secs)?;
    validate_segment_performance_action_script(&value.action_script)
}

fn validate_segment_performance_time_blocks(
    blocks: &[VideoPerformanceTimeBlockV1],
    duration_secs: i32,
) -> Result<(), AppError> {
    if blocks.is_empty() {
        return Err(AppError::InvalidArgument(
            "视频分段规划 performance_plan.time_blocks 不能为空".to_string(),
        ));
    }
    let mut expected_start = 0.0_f64;
    for (index, block) in blocks.iter().enumerate() {
        let field = format!("视频分段规划 performance_plan.time_blocks[{}]", index + 1);
        if !block.start_sec.is_finite() || !block.end_sec.is_finite() {
            return Err(AppError::InvalidArgument(format!(
                "{field}.start_sec/end_sec 必须是有限数字"
            )));
        }
        if (block.start_sec - expected_start).abs() > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "{field}.start_sec 必须从 {expected_start:.2} 秒连续开始，实际为 {:.2}",
                block.start_sec
            )));
        }
        if block.end_sec <= block.start_sec {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 必须大于 start_sec"
            )));
        }
        if block.end_sec - f64::from(duration_secs) > 0.05 {
            return Err(AppError::InvalidArgument(format!(
                "{field}.end_sec 不能超过 duration_secs"
            )));
        }
        for (name, text) in [
            ("camera", block.camera.clone()),
            ("subject_action", block.subject_action.clone()),
            ("continuity_target", block.continuity_target.clone()),
        ] {
            required_text(text, &format!("{field}.{name}"))?;
        }
        expected_start = block.end_sec;
    }
    if (expected_start - f64::from(duration_secs)).abs() > 0.05 {
        return Err(AppError::InvalidArgument(
            "视频分段规划 performance_plan.time_blocks 最后一段 end_sec 必须等于 duration_secs"
                .to_string(),
        ));
    }
    Ok(())
}

fn validate_segment_performance_action_script(
    items: &[VideoPerformanceActionV1],
) -> Result<(), AppError> {
    for (index, item) in items.iter().enumerate() {
        let field = format!("视频分段规划 performance_plan.action_script[{}]", index + 1);
        for (name, text) in [
            ("character", item.character.clone()),
            ("body", item.body.clone()),
            ("movement", item.movement.clone()),
            ("hands", item.hands.clone()),
            ("gaze", item.gaze.clone()),
            ("timing", item.timing.clone()),
        ] {
            required_text(text, &format!("{field}.{name}"))?;
        }
    }
    Ok(())
}
