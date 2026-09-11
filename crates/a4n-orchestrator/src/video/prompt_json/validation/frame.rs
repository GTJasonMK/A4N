use a4n_error::AppError;

use a4n_domain::prompt_language::PromptOutputLanguage;

use super::super::VideoFramePrompt;
use super::prompt_text::{required_min_prompt_text, required_text, validate_reference_label_text};
use super::{PromptTextKind, validate_duration, validate_reference_ids, validate_structured_value};

pub(in crate::video::prompt_json) fn validate_frame_prompt(
    mut segment: VideoFramePrompt,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoFramePrompt, AppError> {
    if segment.frame_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频首尾帧提示词 frame_number 必须大于 0".to_string(),
        ));
    }
    segment.title = required_text(segment.title, "视频首尾帧提示词 title")?;
    segment.scene_summary = required_text(segment.scene_summary, "视频首尾帧提示词 scene_summary")?;
    segment.frame_prompt = required_min_prompt_text(
        segment.frame_prompt,
        "视频首尾帧提示词 frame_prompt",
        prompt_output_language,
        PromptTextKind::FramePrompt,
    )?;
    validate_structured_value(&segment.blocking_state, "视频首尾帧提示词 blocking_state")?;
    validate_structured_value(&segment.lighting_state, "视频首尾帧提示词 lighting_state")?;
    segment.negative_prompt = required_min_prompt_text(
        segment.negative_prompt,
        "视频首尾帧提示词 negative_prompt",
        prompt_output_language,
        PromptTextKind::FrameNegative,
    )?;
    validate_duration(segment.duration_secs, "视频首尾帧提示词 duration_secs")?;
    segment.aspect_ratio = required_text(segment.aspect_ratio, "视频首尾帧提示词 aspect_ratio")?;
    validate_reference_ids(
        &segment.reference_image_ids,
        "视频首尾帧提示词 reference_image_ids",
    )?;
    validate_reference_label_text(
        &segment.frame_prompt,
        "视频首尾帧提示词 frame_prompt",
        segment.reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &segment.negative_prompt,
        "视频首尾帧提示词 negative_prompt",
        segment.reference_image_ids.len(),
    )?;
    validate_segment_numbers(&segment.segment_numbers, "视频首尾帧提示词 segment_numbers")?;
    Ok(segment)
}

fn validate_segment_numbers(numbers: &[i32], field: &str) -> Result<(), AppError> {
    if numbers.is_empty() {
        return Err(AppError::InvalidArgument(format!("{field} 不能为空")));
    }
    if numbers.iter().any(|number| *number <= 0) {
        return Err(AppError::InvalidArgument(format!("{field} 必须为正整数")));
    }
    Ok(())
}
