use super::chain::prompt_stage_label;
use crate::video::VideoDbError;
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
};
use a4n_domain::video_prompt::ChapterVideoPromptRecord;

pub(in crate::video::prompts::consistency::dependencies) fn frame_image_generation_source_for_prompt(
    row: &ChapterVideoPromptRecord,
) -> Option<&'static str> {
    match row.prompt_type.as_str() {
        chapter_video_prompt::PROMPT_TYPE_FRAME
        | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME => {
            Some(IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT)
        }
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME => {
            Some(IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT)
        }
        chapter_video_prompt::PROMPT_TYPE_STORYBOARD => {
            Some(IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT)
        }
        _ => None,
    }
}

pub(in crate::video::prompts::consistency::dependencies) fn ensure_frame_prompt_generation_source_matches(
    prompt: &ChapterVideoPromptRecord,
    generation_source: &str,
) -> Result<&'static str, VideoDbError> {
    let expected_source = frame_image_generation_source_for_prompt(prompt).ok_or_else(|| {
        VideoDbError::InvalidArgument(format!(
            "提示词 #{} 不是可成图的视频帧提示词，不能生成视频帧图片",
            prompt.id
        ))
    })?;
    if generation_source != expected_source {
        return Err(VideoDbError::InvalidArgument(format!(
            "视频帧成图来源 {generation_source} 与{}不匹配",
            prompt_stage_label(prompt)
        )));
    }
    Ok(expected_source)
}

pub(in crate::video::prompts::consistency::dependencies) fn ensure_frame_prompt_workflow_matches_type(
    row: &ChapterVideoPromptRecord,
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
        chapter_video_prompt::PROMPT_TYPE_FRAME => {
            Some(chapter_video_prompt::WORKFLOW_MODE_BOUNDARY_FRAMES)
        }
        chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME => {
            Some(chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN)
        }
        chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME => {
            Some(chapter_video_prompt::WORKFLOW_MODE_CHAT_VIDEO)
        }
        chapter_video_prompt::PROMPT_TYPE_STORYBOARD => {
            Some(chapter_video_prompt::WORKFLOW_MODE_STORYBOARD)
        }
        _ => None,
    }
}
