mod frames;
mod source;

pub(in crate::video::prompt_preview) use frames::{
    placeholder_boundary_frame_context_for_video_prompt, placeholder_first_frame_prompts,
    placeholder_storyboard_prompts,
};
pub(in crate::video::prompt_preview) use source::{
    first_cinematic_shot_for_preview, placeholder_segment_plan, placeholder_segments,
};
