use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerWaitApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunWaitRequest;

#[async_trait::async_trait]
impl AgentWorkerWaitApplicationPort for AgentApplication {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError> {
        AgentApplication::wait_run(self, claimed, reason).await
    }

    async fn wait_run_request(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError> {
        AgentApplication::wait_run_request(self, request).await
    }
}
