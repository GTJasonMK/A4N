mod application;
mod contracts;
mod execution;
mod generation;
mod trace;

#[cfg(test)]
pub(crate) use a4n_api_contract::image_generation::{
    ObjectProfileImageInput, SceneProfileImageInput,
};
pub use application::normalize_image_metadata;
pub use contracts::{
    ImageGenerationExecutionInput, ImageGenerationExecutionOutcome,
    ImageGenerationPostCommitAction, ImageGenerationWorkflowBinding,
};
pub use execution::generated_image_summary;
pub use trace::image_generation_request_trace_json;

#[cfg(test)]
mod tests;
