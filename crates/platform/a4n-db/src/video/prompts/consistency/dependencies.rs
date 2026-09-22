mod deletion;
mod frame_images;
mod graph;
mod mutability;
mod rules;

pub use deletion::delete_chapter_video_prompts_with_policy;
pub(crate) use frame_images::generated_frame_images_for_prompts;
pub use frame_images::{
    effective_frame_prompt_image_reference_ids, ensure_frame_prompt_image_generation_allowed,
};
pub use graph::prompt_downstream_dependencies;
pub use mutability::{
    ensure_chapter_video_prompt_stage_mutable, ensure_chapter_video_prompt_stage_mutable_ignoring,
    ensure_chapter_video_prompt_stages_mutable,
    ensure_chapter_video_prompt_stages_mutable_ignoring,
    ensure_chapter_video_prompt_stages_mutable_skipping_generated_images,
};
