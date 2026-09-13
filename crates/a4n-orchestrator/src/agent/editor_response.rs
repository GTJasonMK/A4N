use super::{AgentApplication, AgentWorkerRun};
use a4n_api_contract::agent::AgentEditorRequest;
use a4n_error::AppError;

/// Runtime-facing boundary for releasing and requeueing editor-backed Runs.
#[async_trait::async_trait]
pub trait AgentEditorResponseApplicationPort: Send + Sync {
    async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &AgentEditorRequest,
    ) -> Result<(), AppError>;

    async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentEditorResponseApplicationPort for AgentApplication {
    async fn wait_for_editor(
        &self,
        claimed: &AgentWorkerRun,
        request: &AgentEditorRequest,
    ) -> Result<(), AppError> {
        AgentApplication::wait_for_editor(self, claimed, request).await
    }

    async fn requeue_after_external_response(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<(), AppError> {
        AgentApplication::requeue_after_external_response(self, claimed).await
    }
}
