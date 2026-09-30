use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStartError},
    inspiration::{PreviewInspirationRequestInput, StartInspirationGenerationInput},
};
use a4n_error::AppError;
use a4n_inspiration_host_contract::{
    InspirationGenerationApplicationPort, InspirationRequestPreviewApplicationPort,
};
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

#[async_trait::async_trait]
impl InspirationRequestPreviewApplicationPort for OrchestratorContext {
    async fn prepare_request(
        &self,
        user_id: i64,
        input: PreviewInspirationRequestInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        self.verify_project_permission(
            input.project_id,
            user_id,
            a4n_domain::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.prepare_inspiration_request_validated(
            input.project_id,
            &input.message,
            prompt_renderer,
        )
        .await
    }
}

#[async_trait::async_trait]
impl InspirationGenerationApplicationPort for OrchestratorContext {
    async fn start_generation(
        &self,
        user_id: i64,
        input: &StartInspirationGenerationInput,
    ) -> Result<crate::GenerationTaskStart, GenerationTaskStartError> {
        OrchestratorContext::start_inspiration_generation_task(self, user_id, input)
            .await
            .map_err(crate::GenerationTaskAccessError::into_start_error)
    }

    async fn run_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartInspirationGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_inspiration_generation_task(self, prompt_renderer, input, handle)
            .await
    }
}
