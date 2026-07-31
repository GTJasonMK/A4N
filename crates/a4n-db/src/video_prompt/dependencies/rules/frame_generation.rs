use super::chain::prompt_stage_label;
use crate::video::VideoDbError;
use a4n_core::models::chapter::chapter_video_prompt;

pub(in crate::video_prompt::dependencies) fn frame_image_generation_source_for_prompt(
    row: &chapter_video_prompt::Model,
) -> Option<&'static str> {
    match row.prompt_type.as_str() {
        "frame_prompt" | "chat_video_frame_prompt" => Some("video_boundary_frame_prompt"),
        "first_frame_prompt" => Some("video_first_frame_prompt"),
        "storyboard_prompt" => Some("video_storyboard_prompt"),
        _ => None,
    }
}

pub(in crate::video_prompt::dependencies) fn ensure_frame_prompt_generation_source_matches(
    prompt: &chapter_video_prompt::Model,
    generation_source: &str,
) -> Result<&'static str, VideoDbError> {
    let expected_source = frame_image_generation_source_for_prompt(prompt).ok_or_else(|| {
        VideoDbError::InvalidArgument(format!(
            "提示词 #{} 不是可成图的视频帧提示词，不能生成视频帧图片",
            prompt.id
        ))
    })?;
    if generation_source.trim() != expected_source {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频帧成图来源 {generation_source} 与{}不匹配",
            prompt_stage_label(prompt)
        )));
    }
    Ok(expected_source)
}

pub(in crate::video_prompt::dependencies) fn ensure_frame_prompt_workflow_matches_type(
    row: &chapter_video_prompt::Model,
) -> Result<(), VideoDbError> {
    let Some(expected_workflow_mode) = expected_prompt_workflow_mode(&row.prompt_type) else {
        return Err(VideoDbError::InvalidArgument(format!(
            "提示词 #{} 类型 {} 不能生成视频帧图片",
            row.id, row.prompt_type
        )));
    };
    if row.workflow_mode != expected_workflow_mode {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频帧提示词 #{} 类型 {} 必须属于 {} workflow，实际为 {}",
            row.id, row.prompt_type, expected_workflow_mode, row.workflow_mode
        )));
    }
    Ok(())
}

fn expected_prompt_workflow_mode(prompt_type: &str) -> Option<&'static str> {
    match prompt_type {
        "frame_prompt" => Some("boundary_frames"),
        "first_frame_prompt" => Some("first_frame_chain"),
        "chat_video_frame_prompt" => Some("chat_video"),
        "storyboard_prompt" => Some("storyboard"),
        _ => None,
    }
}
