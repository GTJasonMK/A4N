use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunWaitRequest;

/// Runtime-facing boundary for releasing a claimed Run into durable waiting.
///
/// Waiting-reason validation, lease fencing and state transition remain in the
/// Agent owner; Runtime supplies only the claimed Run and reason.
#[async_trait::async_trait]
pub trait AgentWorkerWaitApplicationPort: Send + Sync {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError>;

    async fn wait_run_request(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerWaitApplicationPort for AgentApplication {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError> {
        AgentApplication::wait_run(self, claimed, reason).await
    }

    async fn wait_run_request(&self, request: &AgentWorkerRunWaitRequest) -> Result<(), AppError> {
        AgentApplication::wait_run_request(self, request).await
    }
}
