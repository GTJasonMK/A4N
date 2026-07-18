use super::*;
use a4n_core::error::AppError;
use a4n_core::prompt_language::PromptOutputLanguage;
use std::collections::{HashMap, HashSet};

pub(in crate::video) fn parse_chat_video_prompt_shots(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<ChatVideoPromptShot>, AppError> {
    let response: ChatVideoPromptResponse = serde_json::from_str(raw.trim())?;
    if response.shots.is_empty() {
        return Err(AppError::InvalidArgument(
            "Chat 视频提示词响应必须包含至少一个 shots 条目".to_string(),
        ));
    }

    let mut seen_numbers = HashSet::new();
    let ordered_segment_numbers = ordered_segment_numbers(segment_by_number);
    let shot_count = response.shots.len();
    let mut shots = Vec::with_capacity(response.shots.len());
    for (index, input) in response.shots.into_iter().enumerate() {
        let shot_number = derived_prompt_shot_number(
            index,
            shot_count,
            &ordered_segment_numbers,
            "Chat 视频提示词 shot_number",
        )?;
        let segment = segment_by_number.get(&shot_number).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 必须对应已有片段编号",
                shot_number
            ))
        })?;
        let negative_prompt = input.negative_prompt.trim().to_string();
        validate_direct_prompt_text(&negative_prompt, "Chat 视频提示词 negative_prompt")?;
        let shot = ChatVideoPromptShot {
            shot_number,
            title: required_text(input.title, "title")?,
            scene_summary: required_text(input.scene_summary, "scene_summary")?,
            chat_prompt: required_chat_prompt(input.chat_prompt, prompt_output_language)?,
            negative_prompt,
            chat_video: derive_chat_video(segment),
            duration_secs: segment.duration_secs,
            aspect_ratio: segment.aspect_ratio.clone(),
            reference_image_ids: segment.reference_image_ids.clone(),
        };
        let shot =
            validate_chat_video_prompt_shot_against_segment(shot, segment, prompt_output_language)
                .map_err(|err| match err {
                    AppError::InvalidArgument(message) => AppError::InvalidArgument(format!(
                        "Chat 视频提示词 shot_number {} 无效：{}",
                        shot_number, message
                    )),
                    other => other,
                })?;
        if !seen_numbers.insert(shot.shot_number) {
            return Err(AppError::InvalidArgument(format!(
                "Chat 视频提示词 shot_number {} 重复",
                shot.shot_number
            )));
        }
        shots.push(shot);
    }
    Ok(shots)
}

pub(in crate::video) fn validate_chat_video_prompt_shot_against_segment(
    shot: ChatVideoPromptShot,
    segment: &VideoSegmentPlan,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatVideoPromptShot, AppError> {
    validate_direct_prompt_text(&shot.chat_prompt, "Chat 视频提示词 chat_prompt")?;
    validate_prompt_has_no_upload_status(&shot.chat_prompt, "Chat 视频提示词 chat_prompt")?;
    validate_chat_prompt_required_sections(&shot.chat_prompt, prompt_output_language)?;
    validate_reference_ids(
        &shot.reference_image_ids,
        "Chat 视频提示词 reference_image_ids",
    )?;
    validate_chat_video_source_span_ids(&shot.chat_video, &segment.source_span_ids)?;
    validate_chat_video_input_images(&shot.chat_video, &shot.reference_image_ids)?;
    Ok(shot)
}
