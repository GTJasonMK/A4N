use super::{
    frame_images::generated_frame_images_for_prompts,
    rules::{prompt_same_generation_chain, prompt_stage},
};
use crate::video::{self, VideoDbError, prompts::consistency::PromptStage};
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::video_prompt::VideoPromptDownstreamDependencies;
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
    let mut frame_prompts = Vec::new();
    if prompt_stage(prompt) == Some(PromptStage::FramePrompt) {
        frame_prompts.push(prompt);
    }
    collect_prompt_asset_dependencies(prompt, &mut video_ids);

    let rows = video::chapter_video_prompt_sibling_records(
        db,
        prompt.project_id,
        prompt.chapter_id,
        prompt.id,
    )
    .await
    .map_err(VideoDbError::Database)?;
    for row in &rows {
        let Some(later_stage) = prompt_stage(row) else {
            continue;
        };
        if later_stage <= stage || !prompt_same_generation_chain(prompt, row) {
            continue;
        }
        prompt_ids.insert(row.id);
        if prompt_stage(row) == Some(PromptStage::FramePrompt) {
            frame_prompts.push(row);
        }
        collect_prompt_asset_dependencies(row, &mut video_ids);
    }
    for image in
        generated_frame_images_for_prompts(db, prompt.project_id, prompt.chapter_id, &frame_prompts)
            .await?
    {
        image_ids.insert(image.id);
    }

    Ok(VideoPromptDownstreamDependencies {
        prompt_ids: prompt_ids.into_iter().collect(),
        image_ids: image_ids.into_iter().collect(),
        video_ids: video_ids.into_iter().collect(),
    })
}

fn collect_prompt_asset_dependencies(
    prompt: &chapter_video_prompt::Model,
    video_ids: &mut BTreeSet<i64>,
) {
    if prompt_stage(prompt) == Some(PromptStage::Video)
        && let Some(video_id) = prompt.generated_video_id
    {
        video_ids.insert(video_id);
    }
}
