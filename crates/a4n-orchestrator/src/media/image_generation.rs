mod application;
mod contracts;
mod execution;
mod generation;
mod trace;

pub use application::normalize_image_metadata;
pub use contracts::{
    ImageBatchRequestPreviewApplicationPort, ImageCanvasRequestPreviewApplicationPort,
    ImageGenerationApplicationPort, ImageGenerationExecutionInput, ImageGenerationExecutionOutcome,
    ImageGenerationHostPort, ImageGenerationPostCommitAction, ImageGenerationProgress,
    ImageGenerationProviderInput, ImageGenerationProviderOutput,
    ImageGenerationTaskApplicationPort, ImageGenerationWorkflowBinding,
    ImageObjectProfileRequestPreviewApplicationPort, ImagePageRequestPreviewApplicationPort,
    ImageProfilePromptApplicationPort, ImageSceneRequestPreviewApplicationPort,
    ObjectProfileImageInput, ObjectProfileImagePromptOutput, PersistGeneratedImageFileInput,
    PersistedGeneratedImageFile, PrepareGeneratedImageFileInput, PreparedGeneratedImageFile,
    PreparedImageGenerationRequest, SceneProfileImageInput, SceneProfileImagePromptOutput,
};
pub use execution::generated_image_summary;
pub use trace::image_generation_request_trace_json;

#[cfg(test)]
mod tests;
