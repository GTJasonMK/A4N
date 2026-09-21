use super::{AgentApplication, AgentWorkerRun};
use a4n_agent_runtime_contract::AgentWorkerFailureApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerFailureRequest;

#[async_trait::async_trait]
impl AgentWorkerFailureApplicationPort for AgentApplication {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError> {
        AgentApplication::fail_child_run(self, claimed, error_message).await
    }

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError> {
        AgentApplication::fail_worker_run(self, request).await
    }
}
