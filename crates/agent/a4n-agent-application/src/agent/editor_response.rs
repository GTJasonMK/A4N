use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentEditorResponseApplicationPort;
use a4n_api_contract::agent::AgentEditorRequest;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerEditorWaitRequest;

#[async_trait::async_trait]
impl AgentEditorResponseApplicationPort for AgentApplication {
    async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &AgentEditorRequest,
    ) -> Result<(), AppError> {
        AgentApplication::wait_for_editor(self, claimed, request).await
    }

    async fn wait_for_editor_request(
        &self,
        request: &AgentWorkerEditorWaitRequest,
    ) -> Result<(), AppError> {
        AgentApplication::wait_for_editor_request(self, request).await
    }

    async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        AgentApplication::requeue_after_external_response(self, claimed).await
    }
}
