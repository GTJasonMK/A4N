use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerRecoveryApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRecoveryRequest;

#[async_trait::async_trait]
impl AgentWorkerRecoveryApplicationPort for AgentApplication {
    async fn mark_recovery_required(
        &self,
        claimed: &AgentWorkerRun,
        reason: String,
    ) -> Result<(), AppError> {
        AgentApplication::mark_recovery_required(self, claimed, reason)
            .await
            .map(|_| ())
    }

    async fn mark_recovery_required_request(
        &self,
        request: &AgentWorkerRecoveryRequest,
    ) -> Result<(), AppError> {
        AgentApplication::mark_recovery_required_request(self, request).await
    }
}
