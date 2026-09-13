use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentEditorClaimApplicationPort;
use a4n_api_contract::agent::{AgentEditorRequest, RespondAgentEditorInput};
use a4n_error::AppError;

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
