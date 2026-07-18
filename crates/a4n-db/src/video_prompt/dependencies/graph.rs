use super::{
    frame_images::generated_frame_images_for_prompt,
    rules::{prompt_same_generation_chain, prompt_stage},
};
use crate::{
    video::{self, VideoDbError},
    video_prompt::{PromptStage, VideoPromptDownstreamDependencies},
};
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;
use std::collections::BTreeSet;

pub async fn prompt_downstream_dependencies<C>(
    db: &C,
    prompt_id: i64,
) -> Result<VideoPromptDownstreamDependencies, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt = video::chapter_video_prompt_record(db, prompt_id).await?;
    prompt_downstream_dependencies_for_prompt(db, &prompt).await
}

async fn prompt_downstream_dependencies_for_prompt<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
) -> Result<VideoPromptDownstreamDependencies, VideoDbError>
where
    C: ConnectionTrait,
{
    let Some(stage) = prompt_stage(prompt) else {
        return Ok(VideoPromptDownstreamDependencies::default());
    };

    let mut prompt_ids = BTreeSet::<i64>::new();
    let mut image_ids = BTreeSet::<i64>::new();
    let mut video_ids = BTreeSet::<i64>::new();
    collect_prompt_asset_dependencies(db, prompt, &mut image_ids, &mut video_ids).await?;

    let rows = video::chapter_video_prompt_sibling_records(
        db,
        prompt.project_id,
        prompt.chapter_id,
        prompt.id,
    )
    .await
    .map_err(VideoDbError::Database)?;
    for row in rows {
        let Some(later_stage) = prompt_stage(&row) else {
            continue;
        };
        if later_stage <= stage || !prompt_same_generation_chain(prompt, &row) {
            continue;
        }
        prompt_ids.insert(row.id);
        collect_prompt_asset_dependencies(db, &row, &mut image_ids, &mut video_ids).await?;
    }

    Ok(VideoPromptDownstreamDependencies {
        prompt_ids: prompt_ids.into_iter().collect(),
        image_ids: image_ids.into_iter().collect(),
        video_ids: video_ids.into_iter().collect(),
    })
}

async fn collect_prompt_asset_dependencies<C>(
    db: &C,
    prompt: &chapter_video_prompt::Model,
    image_ids: &mut BTreeSet<i64>,
    video_ids: &mut BTreeSet<i64>,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    if prompt_stage(prompt) == Some(PromptStage::FramePrompt) {
        for image in generated_frame_images_for_prompt(db, prompt).await? {
            image_ids.insert(image.id);
        }
    }
    if prompt_stage(prompt) == Some(PromptStage::Video)
        && let Some(video_id) = prompt.generated_video_id
    {
        video_ids.insert(video_id);
    }
    Ok(())
}
