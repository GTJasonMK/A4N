use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerActivationApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;

#[async_trait::async_trait]
impl AgentWorkerActivationApplicationPort for AgentApplication {
    async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        AgentApplication::mark_turn_active(self, claimed).await
    }

    async fn mark_turn_active_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        AgentApplication::mark_turn_active_request(self, request).await
    }
}
