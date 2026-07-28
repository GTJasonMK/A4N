use a4n_orchestrator::media::ImageGenerationProgress as OrchestratorImageGenerationProgress;
pub use a4n_orchestrator::media::{
    GenerateCanvasImageInput, GenerateGalgameImageInput, GeneratedImageSummary,
    ObjectProfileImageInput, ObjectProfileImagePromptOutput, SceneProfileImageInput,
    SceneProfileImagePromptOutput,
};

pub struct ImageGenerationProgress<'a> {
    inner: OrchestratorImageGenerationProgress<'a>,
}

impl<'a> ImageGenerationProgress<'a> {
    pub const fn none() -> Self {
        Self {
            inner: OrchestratorImageGenerationProgress::none(),
        }
    }

    pub(crate) fn inner(&self) -> &OrchestratorImageGenerationProgress<'a> {
        &self.inner
    }
}

pub(in crate::actions::image_generation) static DEFAULT_IMAGE_GENERATION_PROGRESS:
    ImageGenerationProgress<'static> = ImageGenerationProgress::none();
