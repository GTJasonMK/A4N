use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::{
    ChatVideoPromptShot, PromptOutputLanguage, SourceSpan, VideoFramePrompt,
    VideoFramePromptDefaults, VideoPromptShot, VideoSegmentPlan, VideoShootingScriptProjection,
    parse_chat_video_prompt_shots, parse_storyboard_frame_prompts_with_defaults,
    parse_video_frame_prompts_with_defaults, parse_video_prompt_shots,
    parse_video_segment_plan_response,
};

pub(super) fn parse_single_video_prompt_shot(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
    expected_shot_number: i32,
    context: &str,
) -> Result<VideoPromptShot, AppError> {
    let expected_segment = segment_by_number
        .get(&expected_shot_number)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{context} 期望片段 #{} 不存在",
                expected_shot_number
            ))
        })?;
    let scoped_segment = HashMap::from([(expected_shot_number, *expected_segment)]);
    let mut shots = parse_video_prompt_shots(raw, &scoped_segment, prompt_output_language)?;
    if shots.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前片段 1 条 shot，实际返回 {} 条",
            shots.len()
        )));
    }
    let shot = shots.remove(0);
    if shot.shot_number != expected_shot_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 shot_number 必须为 {}，实际为 {}",
            expected_shot_number, shot.shot_number
        )));
    }
    Ok(shot)
}

pub(super) fn parse_single_chat_video_prompt_shot(
    raw: &str,
    segment_by_number: &HashMap<i32, &VideoSegmentPlan>,
    prompt_output_language: PromptOutputLanguage,
    expected_shot_number: i32,
    context: &str,
) -> Result<ChatVideoPromptShot, AppError> {
    let expected_segment = segment_by_number
        .get(&expected_shot_number)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{context} 期望片段 #{} 不存在",
                expected_shot_number
            ))
        })?;
    let scoped_segment = HashMap::from([(expected_shot_number, *expected_segment)]);
    let mut shots = parse_chat_video_prompt_shots(raw, &scoped_segment, prompt_output_language)?;
    if shots.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前片段 1 条 shot，实际返回 {} 条",
            shots.len()
        )));
    }
    let shot = shots.remove(0);
    if shot.shot_number != expected_shot_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 shot_number 必须为 {}，实际为 {}",
            expected_shot_number, shot.shot_number
        )));
    }
    Ok(shot)
}

pub(super) fn parse_single_video_frame_prompt(
    raw: &str,
    defaults: VideoFramePromptDefaults<'_>,
    prompt_output_language: PromptOutputLanguage,
    expected_frame_number: i32,
    context: &str,
) -> Result<VideoFramePrompt, AppError> {
    let mut frames =
        parse_video_frame_prompts_with_defaults(raw, &[defaults], prompt_output_language)?;
    if frames.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前边界帧 1 条 frame，实际返回 {} 条",
            frames.len()
        )));
    }
    let frame = frames.remove(0);
    if frame.frame_number != expected_frame_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 frame_number 必须为 {}，实际为 {}",
            expected_frame_number, frame.frame_number
        )));
    }
    Ok(frame)
}

pub(super) fn parse_single_storyboard_frame_prompt(
    raw: &str,
    defaults: VideoFramePromptDefaults<'_>,
    prompt_output_language: PromptOutputLanguage,
    expected_frame_number: i32,
    context: &str,
) -> Result<VideoFramePrompt, AppError> {
    let mut frames =
        parse_storyboard_frame_prompts_with_defaults(raw, &[defaults], prompt_output_language)?;
    if frames.len() != 1 {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须只返回当前分镜帧 1 条 frame，实际返回 {} 条",
            frames.len()
        )));
    }
    let frame = frames.remove(0);
    if frame.frame_number != expected_frame_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} 返回的 frame_number 必须为 {}，实际为 {}",
            expected_frame_number, frame.frame_number
        )));
    }
    Ok(frame)
}

pub(super) fn parse_generated_segment_plan_with_current_config(
    raw: &str,
    source_spans: &[SourceSpan],
    shooting_script_projection: &VideoShootingScriptProjection,
    duration_secs: i32,
    default_aspect_ratio: &str,
) -> Result<(Value, Vec<VideoSegmentPlan>), AppError> {
    let segment_plan = parse_video_segment_plan_response(
        raw,
        source_spans,
        shooting_script_projection,
        duration_secs,
        default_aspect_ratio,
    )?;
    let segment_json = json!({ "segments": &segment_plan });
    Ok((segment_json, segment_plan))
}
