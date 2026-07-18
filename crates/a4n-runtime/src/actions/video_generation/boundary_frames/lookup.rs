use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;

use super::rules::boundary_frame_numbers_for_prompt;
use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::prompt_chain::{
    image_generation_prompt_id, image_generation_source, prompt_chain_matches,
    prompt_source_type_matches,
};

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::actions::video_generation) struct BoundaryFrameImageIds {
    pub(in crate::actions::video_generation) first: Option<i64>,
    pub(in crate::actions::video_generation) last: Option<i64>,
}

pub(in crate::actions::video_generation) async fn boundary_frame_image_ids_for_prompt(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    prompt: &chapter_video_prompt::Model,
) -> Result<BoundaryFrameImageIds, String> {
    let (first_frame_number, last_frame_number) = boundary_frame_numbers_for_prompt(prompt)?;
    let frame_prompts = state
        .orchestrator
        .chapter_video_boundary_frame_prompt_candidates(
            project_id,
            chapter_id,
            first_frame_number,
            last_frame_number,
        )
        .await
        .map_err(app_error_message)?;

    let first_prompt_id = frame_prompts
        .iter()
        .filter(|frame| frame_prompt_matches_video_prompt(prompt, frame))
        .find(|frame| frame.shot_number == first_frame_number)
        .map(|frame| frame.id);
    let last_prompt_id = frame_prompts
        .iter()
        .filter(|frame| frame_prompt_matches_video_prompt(prompt, frame))
        .find(|frame| frame.shot_number == last_frame_number)
        .map(|frame| frame.id);

    if first_prompt_id.is_none() && last_prompt_id.is_none() {
        return Ok(BoundaryFrameImageIds::default());
    }

    let frame_images = state
        .orchestrator
        .generated_images_for_video_prompt_references(project_id, chapter_id)
        .await
        .map_err(|error| error.to_string())?;

    Ok(BoundaryFrameImageIds {
        first: frame_prompts
            .iter()
            .find(|frame| Some(frame.id) == first_prompt_id)
            .and_then(|frame| latest_boundary_frame_image_id(&frame_images, frame)),
        last: frame_prompts
            .iter()
            .find(|frame| Some(frame.id) == last_prompt_id)
            .and_then(|frame| latest_boundary_frame_image_id(&frame_images, frame)),
    })
}

fn latest_boundary_frame_image_id(
    images: &[generated_image::Model],
    frame_prompt: &chapter_video_prompt::Model,
) -> Option<i64> {
    images
        .iter()
        .find(|image| {
            let metadata = image.metadata_json.as_ref();
            image_generation_prompt_id(metadata) == Some(frame_prompt.id)
                && image_generation_source(metadata) == Some("video_boundary_frame_prompt")
        })
        .map(|image| image.id)
}

pub(super) fn frame_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    frame_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("boundary_frames", video_prompt, frame_prompt)
        && prompt_source_type_matches(video_prompt, "frame_segment_plan")
        && prompt_source_type_matches(frame_prompt, "frame_segment_plan")
}
