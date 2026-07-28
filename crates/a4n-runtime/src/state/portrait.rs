//! Portrait generation runtime facade.

use a4n_api_contract::generation_tasks::GenerationTaskHandle;
use a4n_api_contract::portrait::StartPortraitGenerationInput;
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    portrait::PortraitGenerationHostPort,
};
use a4n_prompt::rendering::PromptRenderer;

pub(crate) struct PortraitRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> PortraitRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn start_portrait_generation_task(
        &self,
        user_id: i64,
        input: StartPortraitGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_portrait_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_portrait_generation_task(
        &self,
        input: StartPortraitGenerationInput,
        handle: GenerationTaskHandle,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn PortraitGenerationHostPort,
    ) -> Result<(), String> {
        self.orchestrator
            .run_portrait_generation_task(input, handle, prompt_renderer, host_port)
            .await
    }
}
