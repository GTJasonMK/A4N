//! 灵感对话 runtime 窄 facade。

use a4n_core::error::AppError;
use a4n_core::models::user::message;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::{
    GenerationTaskAccessError, GenerationTaskStart, OrchestratorContext,
    inspiration::{InspirationGenerationApplicationContext, InspirationPreviewInput},
};

pub(crate) struct InspirationRuntimeFacade<'a> {
    orchestrator: &'a OrchestratorContext,
}

impl<'a> InspirationRuntimeFacade<'a> {
    pub(super) fn new(orchestrator: &'a OrchestratorContext) -> Self {
        Self { orchestrator }
    }

    pub(crate) async fn inspiration_messages(
        &self,
        project_id: i64,
        limit: u64,
    ) -> Result<Vec<message::Model>, AppError> {
        self.orchestrator
            .inspiration_messages(project_id, limit)
            .await
    }

    pub(crate) async fn prepare_inspiration_request_validated(
        &self,
        input: InspirationPreviewInput<'_>,
    ) -> Result<ChatRequest, AppError> {
        self.orchestrator
            .prepare_inspiration_request_validated(input)
            .await
    }

    pub(crate) async fn start_inspiration_generation_task(
        &self,
        user_id: i64,
        input: a4n_api_contract::inspiration::StartInspirationGenerationInput,
    ) -> Result<GenerationTaskStart, GenerationTaskAccessError> {
        self.orchestrator
            .start_inspiration_generation_task(user_id, input)
            .await
    }

    pub(crate) async fn run_inspiration_generation_task(
        &self,
        context: InspirationGenerationApplicationContext<'_>,
        input: a4n_api_contract::inspiration::StartInspirationGenerationInput,
        handle: a4n_api_contract::generation_tasks::GenerationTaskHandle,
    ) -> Result<(), String> {
        self.orchestrator
            .run_inspiration_generation_task(context, input, handle)
            .await
    }
}
