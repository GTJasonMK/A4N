use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;

/// Runtime-facing boundary for releasing a claimed Run into durable waiting.
///
/// Waiting-reason validation, lease fencing and state transition remain in the
/// Agent owner; Runtime supplies only the claimed Run and reason.
#[async_trait::async_trait]
pub trait AgentWorkerWaitApplicationPort: Send + Sync {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerWaitApplicationPort for AgentApplication {
    async fn wait_run(&self, claimed: &AgentWorkerRun, reason: &str) -> Result<(), AppError> {
        AgentApplication::wait_run(self, claimed, reason).await
    }
}
