mod checkpoint;
mod execution;
mod request;
mod workflow;

pub use execution::optimize_manga_prompt;

#[cfg(test)]
pub(super) use checkpoint::manga_prompt_optimization_checkpoint_output;

pub(super) const MANGA_PROMPT_OPTIMIZATION_PROMPT_NAME: &str = "manga_prompt_optimization";
pub(super) const MANGA_PROMPT_OPTIMIZATION_USER_PROMPT_NAME: &str =
    "manga_prompt_optimization_user";
