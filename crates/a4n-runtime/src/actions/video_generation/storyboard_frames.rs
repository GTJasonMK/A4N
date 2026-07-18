use super::prompt_chain::{
    image_generation_prompt_id, image_generation_source, prompt_chain_matches,
    prompt_source_type_matches,
};
use crate::AppState;
use crate::actions::app_error_message;
use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;

pub(super) async fn storyboard_image_id_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    let storyboard_prompts = state
        .orchestrator
        .chapter_video_storyboard_prompt_candidates(project_id, chapter_id, prompt.shot_number)
        .await
        .map_err(app_error_message)?;

    let Some(storyboard_prompt) = storyboard_prompts.iter().find(|storyboard_prompt| {
        storyboard_prompt_matches_video_prompt(prompt, storyboard_prompt)
    }) else {
        return Ok(None);
    };

    let frame_images = state
        .orchestrator
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await
        .map_err(|error| error.to_string())?;

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
