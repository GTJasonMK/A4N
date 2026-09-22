mod generation_gate;
mod inputs;
mod lookup;
mod resume;

pub(in crate::video::prompts::generation::frame_prompts) use generation_gate::ensure_video_prompt_generation_can_start;
pub(in crate::video::prompts::generation::frame_prompts) use inputs::{
    reusable_frame_rows_for_video_prompt_inputs, validate_frame_prompts_for_video_prompt_inputs,
};
pub(in crate::video) use lookup::boundary_frame_rows_for_image_lookup;
#[cfg(test)]
pub(in crate::video::prompts::generation::frame_prompts) use lookup::frame_row_matches_image_lookup_context;
pub(in crate::video::prompts::generation::frame_prompts) use resume::reusable_frame_rows_for_resume;
