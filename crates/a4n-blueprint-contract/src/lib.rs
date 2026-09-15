//! Runtime-facing application contracts for blueprint workflows.

use a4n_api_contract::{
    blueprint::{
        PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput,
        StartBlueprintGenerationInput, StartBlueprintRefinementInput,
    },
    generation_tasks::{GenerationTaskHandle, GenerationTaskStart, GenerationTaskStartError},
    workflow::WorkflowResumeRecord,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::{blueprint_generation::BlueprintGenerationPreviewStep, rendering::PromptRenderer};

/// Host-facing boundary for blueprint generation request previews.
#[async_trait::async_trait]
pub trait BlueprintGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generation(
        &self,
        user_id: i64,
        input: PreviewBlueprintGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<BlueprintGenerationPreviewStep, AppError>;
}

/// Host-facing boundary for blueprint refinement request previews.
#[async_trait::async_trait]
pub trait BlueprintRefinementPreviewApplicationPort: Send + Sync {
    async fn preview_refinement(
        &self,
        user_id: i64,
        input: PreviewBlueprintRefinementInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

/// Host-facing boundary for blueprint generation workers.
#[async_trait::async_trait]
pub trait BlueprintGenerationApplicationPort: Send + Sync {
    fn generation_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintGenerationInput, AppError>;

    async fn start_generation(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

/// Host-facing boundary for blueprint refinement workers.
#[async_trait::async_trait]
pub trait BlueprintRefinementApplicationPort: Send + Sync {
    fn refinement_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintRefinementInput, AppError>;

    async fn start_refinement(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<GenerationTaskStart, GenerationTaskStartError>;

    async fn run_refinement(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintRefinementInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}
