use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerLeaseApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;

#[async_trait::async_trait]
impl AgentWorkerLeaseApplicationPort for AgentApplication {
    async fn renew_worker_lease(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        AgentApplication::renew_worker_lease(self, claimed).await
    }

    async fn renew_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        AgentApplication::renew_worker_lease_request(self, request).await
    }
}
