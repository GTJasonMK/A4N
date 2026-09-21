use super::AgentApplication;
use a4n_agent_runtime_contract::AgentApprovalExpiryApplicationPort;
use a4n_error::AppError;

#[async_trait::async_trait]
impl AgentApprovalExpiryApplicationPort for AgentApplication {
    async fn expire_pending_approvals(&self, limit: u64) -> Result<usize, AppError> {
        Ok(AgentApplication::expire_pending_approvals(self, limit)
            .await?
            .len())
    }
}
