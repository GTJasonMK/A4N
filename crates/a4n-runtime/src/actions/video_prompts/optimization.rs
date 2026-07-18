mod api;
mod checkpoint;
mod contracts;
mod request;
mod rules;

pub use api::optimize_chapter_video_prompt;
pub use contracts::OptimizeChapterVideoPromptInput;

#[cfg(test)]
pub(super) use checkpoint::video_prompt_optimization_checkpoint_output;
#[cfg(test)]
pub(super) use contracts::OptimizedVideoPromptFields;
#[cfg(test)]
pub(super) use request::build_video_prompt_optimization_request;
#[cfg(test)]
pub(super) use rules::{optimized_motion_prompt, validate_optimized_prompt_fields};
