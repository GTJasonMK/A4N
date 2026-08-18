mod application;
mod contracts;
mod execution;
mod generation;
mod trace;

pub use application::normalize_image_metadata;
pub use contracts::{
    GenerateBatchImagesInput, GenerateCanvasImageInput, GenerateGalgameImageInput,
    GeneratePageImageInput, GenerateSceneImageInput, GeneratedImageSummary,
    ImageGenerationExecutionInput, ImageGenerationExecutionOutcome, ImageGenerationHostPort,
    ImageGenerationPostCommitAction, ImageGenerationProgress, ImageGenerationProviderInput,
    ImageGenerationProviderOutput, ImageGenerationWorkflowBinding, ObjectProfileImageInput,
    ObjectProfileImagePromptOutput, PersistGeneratedImageFileInput, PersistedGeneratedImageFile,
    PreparedImageGenerationRequest, SceneProfileImageInput, SceneProfileImagePromptOutput,
};
pub use execution::generated_image_summary;
pub use trace::image_generation_request_trace_json;

#[cfg(test)]
mod tests;
