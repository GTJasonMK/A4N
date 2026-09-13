use super::{AgentApplication, AgentRunContext, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerContextApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerContextSummaryRequest;
use serde_json::Value;

#[async_trait::async_trait]
impl AgentWorkerContextApplicationPort for AgentApplication {
    async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        AgentApplication::load_run_context(self, claimed).await
    }

    async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        AgentApplication::update_context_summary(self, claimed, context_summary).await
    }

    async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        AgentApplication::update_context_summary_request(self, request).await
    }
}
