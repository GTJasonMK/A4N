use std::collections::HashSet;

use a4n_error::AppError;

use crate::video::{
    FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT,
    library::ensure_video_reference_ids_known,
    prompt_json::{VideoFramePrompt, VideoSegmentPlan, validate_reference_label_text},
};

use super::boundary::ensure_no_storyboard_panels;

pub(in crate::video) fn validate_first_frame_prompts(
    first_frame_prompts: &[VideoFramePrompt],
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
) -> Result<(), AppError> {
    if first_frame_prompts.len() != FIRST_FRAME_CHAIN_INITIAL_FRAME_COUNT {
        return Err(AppError::InvalidArgument(format!(
            "首帧串联模式只允许 1 条初始首帧提示词：当前 {} 条",
            first_frame_prompts.len()
        )));
    }
    let first_segment = segment_plan.first().ok_or_else(|| {
        AppError::InvalidArgument("首帧串联片段规划至少需要 1 个片段".to_string())
    })?;
    let frame = &first_frame_prompts[0];
    if frame.frame_number != 1 {
        return Err(AppError::InvalidArgument(format!(
            "初始首帧提示词 frame_number 必须为 1，实际为 {}",
            frame.frame_number
        )));
    }
    if frame.segment_numbers != vec![first_segment.segment_number] {
        return Err(AppError::InvalidArgument(format!(
            "初始首帧提示词 segment_numbers 必须且只能包含第 1 个片段编号 {}",
            first_segment.segment_number
        )));
    }
    if frame.duration_secs < 1 {
        return Err(AppError::InvalidArgument(format!(
            "初始首帧提示词时长必须大于 0，实际为 {}s",
            frame.duration_secs
        )));
    }
    ensure_video_reference_ids_known(
        &frame.reference_image_ids,
        allowed_reference_ids,
        "初始首帧",
    )?;
    validate_reference_label_text(
        &frame.frame_prompt,
        "初始首帧 frame_prompt",
        frame.reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &frame.negative_prompt,
        "初始首帧 negative_prompt",
        frame.reference_image_ids.len(),
    )?;
    ensure_no_storyboard_panels(frame, "初始首帧")?;
    Ok(())
}
