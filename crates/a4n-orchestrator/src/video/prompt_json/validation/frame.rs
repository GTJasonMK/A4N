use a4n_core::error::AppError;

use a4n_core::prompt_language::PromptOutputLanguage;

use super::super::VideoFramePrompt;
use super::prompt_text::{required_min_prompt_text, required_text, validate_reference_label_text};
use super::{PromptTextKind, validate_duration, validate_reference_ids, validate_structured_value};

pub(in crate::video::prompt_json) fn validate_frame_prompt(
    segment: VideoFramePrompt,
    prompt_output_language: PromptOutputLanguage,
) -> Result<VideoFramePrompt, AppError> {
    let VideoFramePrompt {
        frame_number,
        title,
        scene_summary,
        storyboard_panels,
        frame_prompt,
        blocking_state,
        lighting_state,
        negative_prompt,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        segment_numbers,
    } = segment;
    if frame_number < 1 {
        return Err(AppError::InvalidArgument(
            "视频首尾帧提示词 frame_number 必须大于 0".to_string(),
        ));
    }
    let title = required_text(title, "视频首尾帧提示词 title")?;
    let scene_summary = required_text(scene_summary, "视频首尾帧提示词 scene_summary")?;
    let frame_prompt = required_min_prompt_text(
        frame_prompt,
        "视频首尾帧提示词 frame_prompt",
        prompt_output_language,
        PromptTextKind::FramePrompt,
    )?;
    validate_structured_value(&blocking_state, "视频首尾帧提示词 blocking_state")?;
    validate_structured_value(&lighting_state, "视频首尾帧提示词 lighting_state")?;
    let negative_prompt = required_min_prompt_text(
        negative_prompt,
        "视频首尾帧提示词 negative_prompt",
        prompt_output_language,
        PromptTextKind::FrameNegative,
    )?;
    validate_duration(duration_secs, "视频首尾帧提示词 duration_secs")?;
    let aspect_ratio = required_text(aspect_ratio, "视频首尾帧提示词 aspect_ratio")?;
    validate_reference_ids(&reference_image_ids, "视频首尾帧提示词 reference_image_ids")?;
    validate_reference_label_text(
        &frame_prompt,
        "视频首尾帧提示词 frame_prompt",
        reference_image_ids.len(),
    )?;
    validate_reference_label_text(
        &negative_prompt,
        "视频首尾帧提示词 negative_prompt",
        reference_image_ids.len(),
    )?;
    validate_segment_numbers(&segment_numbers, "视频首尾帧提示词 segment_numbers")?;
    Ok(VideoFramePrompt {
        frame_number,
        title,
        scene_summary,
        storyboard_panels,
        frame_prompt,
        blocking_state,
        lighting_state,
        negative_prompt,
        duration_secs,
        aspect_ratio,
        reference_image_ids,
        segment_numbers,
    })
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
