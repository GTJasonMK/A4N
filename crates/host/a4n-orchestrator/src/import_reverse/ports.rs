use a4n_api_contract::import_reverse::{ReverseEngineerInput, ReverseEngineeringCandidate};
use a4n_error::AppError;
use a4n_novel_host_contract::{
    ImportReverseGenerationApplicationPort, ImportReversePreviewApplicationPort,
};
use a4n_prompt::rendering::PromptRenderer;

use crate::OrchestratorContext;

use super::ReverseEngineeringTarget;

#[async_trait::async_trait]
impl a4n_api_contract::import_reverse::ImportReverseReadApplicationPort for OrchestratorContext {
    async fn latest(
        &self,
        project_id: i64,
        target: ReverseEngineeringTarget,
    ) -> Result<Option<ReverseEngineeringCandidate>, AppError> {
        self.latest_reverse_engineering_candidate_validated(project_id, target)
            .await
    }
}

#[async_trait::async_trait]
impl ImportReversePreviewApplicationPort for OrchestratorContext {
    async fn prepare_request(
        &self,
        input: ReverseEngineerInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm_provider_contract::provider::ChatRequest, AppError> {
        self.prepare_reverse_engineer_request_validated(input, prompt_renderer)
            .await
    }
}

impl ImportReverseGenerationApplicationPort for OrchestratorContext {
    fn run<'a>(
        &'a self,
        input: ReverseEngineerInput,
        prompt_renderer: &'a dyn PromptRenderer,
        progress: &'a (dyn for<'b> Fn(u32, u32, &'b str) -> Result<(), AppError> + Send + Sync),
        active_task_id: Option<&'a str>,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<ReverseEngineeringCandidate, AppError>>
                + Send
                + 'a,
        >,
    > {
        Box::pin(self.run_reverse_engineering_validated(
            input,
            prompt_renderer,
            progress,
            active_task_id,
        ))
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::import_reverse::ImportReverseConfirmationApplicationPort
    for OrchestratorContext
{
    async fn confirm(&self, workflow_run_id: i64) -> Result<ReverseEngineeringCandidate, AppError> {
        self.confirm_reverse_engineering_candidate_validated(workflow_run_id)
            .await
    }
}
