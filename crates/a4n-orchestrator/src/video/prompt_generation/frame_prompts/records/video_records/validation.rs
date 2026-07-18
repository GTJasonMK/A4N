use std::collections::{HashMap, HashSet};

use a4n_core::error::AppError;

use super::super::super::super::super::{
    PromptOutputLanguage,
    prompt_json::{
        ChatVideoPromptShot, VideoPromptShot, VideoSegmentPlan,
        validate_chat_video_prompt_shot_against_segment,
        validate_video_prompt_shot_against_segment,
    },
};

pub(in crate::video) fn validate_video_prompts(
    video_prompts: &[VideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    if video_prompts.len() != segment_by_number.len() {
        return Err(AppError::InvalidArgument(format!(
            "视频提示词数量必须等于片段数：当前 {} 条，片段数 {}",
            video_prompts.len(),
            segment_by_number.len()
        )));
    }
    validate_partial_video_prompts(video_prompts, segment_by_number, prompt_output_language)
}

pub(in crate::video) fn validate_partial_video_prompts(
    video_prompts: &[VideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for prompt in video_prompts {
        let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 必须对应已有片段编号",
                prompt.shot_number
            ))
        })?;
        validate_video_prompt_shot_against_segment(prompt.clone(), segment, prompt_output_language)
            .map_err(|error| match error {
                AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                    "视频提示词 shot_number {} 无效：{}",
                    prompt.shot_number, message
                )),
                other => other,
            })?;
        if !seen.insert(prompt.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "视频提示词 shot_number {} 重复",
                prompt.shot_number
            )));
        }
    }
    Ok(())
}

pub(in crate::video) fn validate_chat_video_prompts(
    video_prompts: &[ChatVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    if video_prompts.len() != segment_by_number.len() {
        return Err(AppError::InvalidArgument(format!(
            "Chat 视频提示词数量必须等于片段数：当前 {} 条，片段数 {}",
            video_prompts.len(),
            segment_by_number.len()
        )));
    }
    validate_partial_chat_video_prompts(video_prompts, segment_by_number, prompt_output_language)
}

pub(in crate::video) fn validate_partial_chat_video_prompts(
    video_prompts: &[ChatVideoPromptShot],
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<(), AppError> {
    let mut seen = HashSet::new();
    for prompt in video_prompts {
        let segment = segment_by_number.get(&prompt.shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 必须对应已有片段编号",
                prompt.shot_number
            ))
        })?;
        validate_chat_video_prompt_shot_against_segment(
            prompt.clone(),
            segment,
            prompt_output_language,
        )
        .map_err(|error| match error {
            AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 无效：{}",
                prompt.shot_number, message
            )),
            other => other,
        })?;
        if !seen.insert(prompt.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 重复",
                prompt.shot_number
            )));
        }
    }
    Ok(())
}
