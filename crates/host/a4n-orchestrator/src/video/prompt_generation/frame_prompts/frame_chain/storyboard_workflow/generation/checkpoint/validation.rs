use std::collections::HashSet;

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_error::AppError;

use crate::video::prompt_generation::frame_prompts::{
    apply_boundary_frame_style_template, ensure_video_reference_ids_known,
    parse_single_storyboard_frame_prompt,
};
use crate::video::prompt_json::{VideoFramePrompt, VideoFramePromptDefaults};

pub(super) struct StoryboardFrameValidationInput<'a> {
    pub raw: &'a str,
    pub frame_number: i32,
    pub duration_secs: i32,
    pub default_aspect_ratio: &'a str,
    pub required_aspect_ratio: &'a str,
    pub expected_segment_numbers: &'a [i32],
    pub reference_image_ids: &'a [i64],
    pub allowed_reference_ids: &'a HashSet<i64>,
    pub style: &'a str,
    pub prompt_output_language: PromptOutputLanguage,
}

pub(super) fn parse_and_validate_storyboard_frame(
    input: StoryboardFrameValidationInput<'_>,
) -> Result<VideoFramePrompt, AppError> {
    let StoryboardFrameValidationInput {
        raw,
        frame_number,
        duration_secs,
        default_aspect_ratio,
        required_aspect_ratio,
        expected_segment_numbers,
        reference_image_ids,
        allowed_reference_ids,
        style,
        prompt_output_language,
    } = input;
    let mut frame_prompt = parse_single_storyboard_frame_prompt(
        raw,
        VideoFramePromptDefaults {
            frame_number,
            duration_secs,
            aspect_ratio: default_aspect_ratio,
            segment_numbers: expected_segment_numbers.to_vec(),
            reference_image_ids: reference_image_ids.to_vec(),
        },
        prompt_output_language,
        frame_number,
        "分镜提示词",
    )?;
    apply_boundary_frame_style_template(&mut frame_prompt, style, prompt_output_language);
    if frame_prompt.segment_numbers != expected_segment_numbers {
        return Err(AppError::InvalidArgument(format!(
            "分镜提示词 #{} segment_numbers 必须为 {:?}，实际为 {:?}",
            frame_number, expected_segment_numbers, frame_prompt.segment_numbers
        )));
    }
    if frame_prompt.duration_secs != duration_secs {
        return Err(AppError::InvalidArgument(format!(
            "分镜提示词 #{} duration_secs 必须为 {}，实际为 {}",
            frame_number, duration_secs, frame_prompt.duration_secs
        )));
    }
    if frame_prompt.aspect_ratio != required_aspect_ratio {
        return Err(AppError::InvalidArgument(format!(
            "分镜提示词 #{} aspect_ratio 必须为 {}，实际为 {}",
            frame_number, required_aspect_ratio, frame_prompt.aspect_ratio
        )));
    }
    ensure_video_reference_ids_known(
        &frame_prompt.reference_image_ids,
        allowed_reference_ids,
        &format!("视频分镜 #{}", frame_number),
    )?;
    frame_prompt.frame_number = frame_number;
    Ok(frame_prompt)
}
