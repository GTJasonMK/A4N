use super::{AgentApplication, AgentWorkerRun};
use a4n_api_contract::agent::{AgentEditorRequest, RespondAgentEditorInput};
use a4n_error::AppError;

/// Runtime-facing boundary for claiming one editor response request.
///
/// Editor ownership, request validation and lease creation remain in the
/// Agent owner; Runtime receives only the claimed Run and request snapshot.
#[async_trait::async_trait]
pub trait AgentEditorClaimApplicationPort: Send + Sync {
    async fn claim_editor_request(
        &self,
        user_id: i64,
        input: &RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, AgentEditorRequest), AppError>;
}

#[async_trait::async_trait]
impl AgentEditorClaimApplicationPort for AgentApplication {
    async fn claim_editor_request(
        &self,
        user_id: i64,
        input: &RespondAgentEditorInput,
    ) -> Result<(AgentWorkerRun, AgentEditorRequest), AppError> {
        AgentApplication::claim_editor_request(self, user_id, input).await
    }
}
