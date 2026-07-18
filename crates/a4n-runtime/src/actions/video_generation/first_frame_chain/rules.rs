use a4n_core::models::ai::generated_image;
use a4n_core::models::chapter::chapter_video_prompt;

use crate::actions::video_generation::prompt_chain::{
    image_generation_prompt_id, image_generation_shot_number, image_generation_source,
    prompt_chain_matches, prompt_source_type_matches,
};

pub(super) fn first_frame_prompt_matches_video_prompt(
    video_prompt: &chapter_video_prompt::Model,
    first_frame_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("first_frame_chain", video_prompt, first_frame_prompt)
        && prompt_uses_shared_segment_source(video_prompt)
        && prompt_uses_shared_segment_source(first_frame_prompt)
}

pub(super) fn previous_video_prompt_matches_current(
    current_prompt: &chapter_video_prompt::Model,
    previous_prompt: &chapter_video_prompt::Model,
) -> bool {
    prompt_chain_matches("first_frame_chain", current_prompt, previous_prompt)
        && current_prompt.prompt_type == "video_prompt"
        && previous_prompt.prompt_type == "video_prompt"
        && previous_prompt.shot_number == current_prompt.shot_number - 1
        && prompt_uses_shared_segment_source(current_prompt)
        && prompt_uses_shared_segment_source(previous_prompt)
}

fn prompt_uses_shared_segment_source(prompt: &chapter_video_prompt::Model) -> bool {
    prompt_source_type_matches(prompt, "frame_segment_plan")
}

pub(super) fn tail_frame_image_matches_prompt(
    image: &generated_image::Model,
    previous_prompt: &chapter_video_prompt::Model,
) -> bool {
    let metadata = image.metadata_json.as_ref();
    image_generation_source(metadata) == Some("video_tail_frame_capture")
        && image_generation_prompt_id(metadata) == Some(previous_prompt.id)
        && image_generation_shot_number(metadata) == Some(previous_prompt.shot_number)
}
