use a4n_core::models::chapter::chapter_video_prompt;

use crate::AppState;
use crate::actions::app_error_message;

use super::rules::{
    first_frame_prompt_matches_video_prompt, previous_video_prompt_matches_current,
    tail_frame_image_matches_prompt,
};
use crate::actions::video_generation::prompt_chain::{
    image_generation_prompt_id, image_generation_source,
};

pub(in crate::actions::video_generation) async fn first_frame_chain_start_image_id_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    if prompt.shot_number <= 1 {
        return latest_generated_first_frame_image_id(state, project_id, chapter_id, prompt).await;
    }
    let Some(previous_prompt) =
        latest_previous_video_prompt(state, project_id, chapter_id, prompt).await?
    else {
        return Ok(None);
    };
    latest_tail_frame_image_id_for_prompt(state, project_id, chapter_id, &previous_prompt).await
}

async fn latest_generated_first_frame_image_id(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    video_prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    let first_frame_prompt = state
        .orchestrator
        .chapter_video_first_frame_prompt_candidates(project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    let Some(first_frame_prompt) = first_frame_prompt
        .iter()
        .find(|frame| first_frame_prompt_matches_video_prompt(video_prompt, frame))
    else {
        return Ok(None);
    };
    let images = state
        .orchestrator
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(first_frame_prompt.id)
                && image_generation_source(metadata) == Some("video_first_frame_prompt")
        })
        .map(|image| image.id))
}

async fn previous_video_prompt_candidates(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, String> {
    state
        .orchestrator
        .chapter_video_first_frame_chain_prompt_candidates(project_id, chapter_id)
        .await
        .map_err(app_error_message)
}

async fn latest_previous_video_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    current_prompt: &chapter_video_prompt::Model,
) -> Result<Option<chapter_video_prompt::Model>, String> {
    Ok(
        previous_video_prompt_candidates(state, project_id, chapter_id)
            .await?
            .into_iter()
            .find(|candidate| previous_video_prompt_matches_current(current_prompt, candidate)),
    )
}

async fn latest_tail_frame_image_id_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    previous_prompt: &chapter_video_prompt::Model,
) -> Result<Option<i64>, String> {
    let images = state
        .orchestrator
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await
        .map_err(|error| error.to_string())?;
    Ok(images
        .iter()
        .find(|image| tail_frame_image_matches_prompt(image, previous_prompt))
        .map(|image| image.id))
}
