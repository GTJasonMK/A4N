use super::AgentApplication;
use a4n_agent_runtime_contract::AgentRunLeaseRecoveryApplicationPort;
use a4n_error::AppError;

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
