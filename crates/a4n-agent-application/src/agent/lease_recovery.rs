use super::AgentApplication;
use a4n_error::AppError;

/// Runtime-facing boundary for recovering Agent runs whose worker lease expired.
///
/// The owner keeps the lease recheck, fencing and recovery state transition;
/// Runtime only needs the bounded scan result.
#[async_trait::async_trait]
pub trait AgentRunLeaseRecoveryApplicationPort: Send + Sync {
    async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<usize, AppError>;
}

#[async_trait::async_trait]
impl AgentRunLeaseRecoveryApplicationPort for AgentApplication {
    async fn recover_expired_runs(
        &self,
        limit: u64,
        error_message: Option<&str>,
    ) -> Result<usize, AppError> {
        Ok(
            AgentApplication::recover_expired_runs(self, limit, error_message)
                .await?
                .len(),
        )
    }
}
