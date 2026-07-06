use super::prompt_chain::{
    image_generation_prompt_id, image_generation_source, prompt_chain_matches,
    prompt_source_type_matches,
};
use crate::AppState;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::*;

pub(super) async fn storyboard_image_id_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    let storyboard_prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("storyboard"))
        .filter(chapter_video_prompt::Column::PromptType.eq("storyboard_prompt"))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(prompt.shot_number))
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    let Some(storyboard_prompt) = storyboard_prompts.iter().find(|storyboard_prompt| {
        storyboard_prompt_matches_video_prompt(prompt, storyboard_prompt)
    }) else {
        return Ok(None);
    };

    let frame_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .filter(generated_image::Column::ChapterId.eq(chapter_id))
        .order_by_desc(generated_image::Column::Id)
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(latest_storyboard_image_id(&frame_images, storyboard_prompt))
}

fn storyboard_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    storyboard_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("storyboard", video_prompt, storyboard_prompt)
        && prompt_source_type_matches(video_prompt, "frame_segment_plan")
        && prompt_source_type_matches(storyboard_prompt, "frame_segment_plan")
}

fn latest_storyboard_image_id(
    images: &[generated_image::Model],
    storyboard_prompt: &chapter_video_prompt::Model,
) -> Option<i64> {
    images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(storyboard_prompt.id)
                && image_generation_source(metadata) == Some("video_storyboard_prompt")
        })
        .map(|image| image.id)
}
