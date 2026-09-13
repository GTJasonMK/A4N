use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerClaimApplicationPort;
use a4n_error::AppError;

#[async_trait::async_trait]
impl AgentWorkerClaimApplicationPort for AgentApplication {
    async fn claim_worker_run(&self, worker_id: &str) -> Result<Option<AgentWorkerRun>, AppError> {
        AgentApplication::claim_worker_run(self, worker_id).await
    }
}
