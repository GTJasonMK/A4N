use std::collections::BTreeMap;

use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;

use super::super::super::super::super::prompt_json::ChatVideoPromptShot;
use super::metadata::{
    SavedChatVideoPromptMetadata, required_row_aspect_ratio, required_row_text, row_metadata,
    row_reference_image_ids,
};

pub(in crate::video) fn reusable_chat_video_prompts_from_rows(
    rows: &[chapter_video_prompt::Model],
) -> Result<BTreeMap<i32, ChatVideoPromptShot>, AppError> {
    let mut by_shot = BTreeMap::<i32, ChatVideoPromptShot>::new();
    for row in rows {
        let prompt = reusable_chat_video_prompt_from_row(row)?;
        by_shot.entry(row.shot_number).or_insert(prompt);
    }
    Ok(by_shot)
}

fn reusable_chat_video_prompt_from_row(
    row: &chapter_video_prompt::Model,
) -> Result<ChatVideoPromptShot, AppError> {
    let metadata = row_metadata::<SavedChatVideoPromptMetadata>(row, "Chat 视频提示词")?;
    let aspect_ratio = required_row_aspect_ratio(row, "Chat 视频提示词")?;
    let reference_image_ids = row_reference_image_ids(row, "Chat 视频提示词")?;
    Ok(ChatVideoPromptShot {
        shot_number: row.shot_number,
        title: required_row_text(&row.title, row, "Chat 视频提示词", "title")?,
        scene_summary: required_row_text(
            &row.scene_summary,
            row,
            "Chat 视频提示词",
            "scene_summary",
        )?,
        chat_prompt: row.visual_prompt.clone(),
        negative_prompt: row.negative_prompt.clone().unwrap_or_default(),
        chat_video: metadata.chat_video,
        duration_secs: row.duration_secs,
        aspect_ratio,
        reference_image_ids,
    })
}
