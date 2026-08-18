use super::{
    frame_images::generated_frame_images_for_prompt,
    rules::{prompt_same_generation_chain, prompt_stage, prompt_stage_label},
};
use crate::{
    video::{self, VideoDbError},
    video_prompt::PromptStage,
};
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;
use std::collections::BTreeSet;

pub async fn ensure_chapter_video_prompt_stage_mutable<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    ensure_chapter_video_prompt_stage_mutable_ignoring(db, prompt, operation, &BTreeSet::new())
        .await
}

pub async fn ensure_chapter_video_prompt_stage_mutable_ignoring<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
    ignored_prompt_ids: &BTreeSet<i64>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let Some(stage) = prompt_stage(prompt) else {
        return Ok(());
    };
    if stage == PromptStage::Video && prompt.generated_video_id.is_some() {
        return Err(VideoDbError::InvalidArgument(format!(
            "已存在由该视频提示词生成的视频，不能{operation}视频提示词；请先删除对应视频"
        )));
    }
    if stage == PromptStage::FramePrompt {
        ensure_frame_prompt_has_no_generated_image(db, prompt, operation).await?;
    }

    let rows = video::chapter_video_prompt_sibling_records(
        db,
        prompt.project_id,
        prompt.chapter_id,
        prompt.id,
    )
    .await
    .map_err(VideoDbError::Database)?;
    if let Some(later) = rows
        .iter()
        .filter(|row| !ignored_prompt_ids.contains(&row.id))
        .filter(|row| prompt_same_generation_chain(prompt, row))
        .filter_map(|row| {
            prompt_stage(row)
                .filter(|later_stage| *later_stage > stage)
                .map(|later_stage| (later_stage, row))
        })
        .min_by_key(|(later_stage, _)| *later_stage)
        .map(|(_, row)| row)
    {
        let current_label = prompt_stage_label(prompt);
        let later_label = prompt_stage_label(later);
        return Err(VideoDbError::InvalidArgument(format!(
            "已存在后续{later_label}，不能{operation}{current_label}；请先删除后续{later_label}"
        )));
    }
    Ok(())
}

pub async fn chapter_video_prompt_has_generated_frame_image<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
) -> Result<bool, VideoDbError>
where
    C: ConnectionTrait,
{
    Ok(!generated_frame_images_for_prompt(db, prompt)
        .await?
        .is_empty())
}

pub(super) async fn ensure_frame_prompt_has_no_generated_image<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    operation: &str,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    if let Some(image) = generated_frame_images_for_prompt(db, prompt)
        .await?
        .into_iter()
        .next()
    {
        let current_label = prompt_stage_label(prompt);
        let image_label = match prompt.prompt_type.as_str() {
            chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME => "首帧图片",
            chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME => "Chat 首尾帧图片",
            chapter_video_prompt::PROMPT_TYPE_STORYBOARD => "分镜板图片",
            _ => "边界帧图片",
        };
        return Err(VideoDbError::InvalidArgument(format!(
            "已存在由该{current_label}生成的{image_label} #{}，不能{operation}{current_label}；请先删除对应图片",
            image.id
        )));
    }
    Ok(())
}
