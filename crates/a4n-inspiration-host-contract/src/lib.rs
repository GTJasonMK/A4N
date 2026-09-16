//! Runtime-facing contracts for inspiration generation.

use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError},
    inspiration::{PreviewInspirationRequestInput, StartInspirationGenerationInput},
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt_contract::rendering::PromptRenderer;
use async_trait::async_trait;

#[async_trait]
pub trait InspirationRequestPreviewApplicationPort: Send + Sync {
    async fn prepare_request(
        &self,
        user_id: i64,
        input: PreviewInspirationRequestInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

#[async_trait]
pub trait InspirationGenerationApplicationPort: Send + Sync {
    async fn start_generation(
        &self,
        user_id: i64,
        input: &StartInspirationGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartInspirationGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}
