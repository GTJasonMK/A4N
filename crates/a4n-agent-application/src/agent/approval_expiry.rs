use super::AgentApplication;
use a4n_error::AppError;

/// Runtime-facing boundary for the bounded Agent approval expiry scan.
#[async_trait::async_trait]
pub trait AgentApprovalExpiryApplicationPort: Send + Sync {
    async fn expire_pending_approvals(&self, limit: u64) -> Result<usize, AppError>;
}

#[async_trait::async_trait]
impl AgentApprovalExpiryApplicationPort for AgentApplication {
    async fn expire_pending_approvals(&self, limit: u64) -> Result<usize, AppError> {
        Ok(AgentApplication::expire_pending_approvals(self, limit)
            .await?
            .len())
    }
}
