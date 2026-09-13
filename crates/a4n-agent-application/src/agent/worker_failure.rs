use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerFailureRequest;

/// Runtime-facing boundary for persisting a worker failure.
///
/// Parent delegation recovery and lease-fenced terminal transitions remain in
/// the Agent owner; Runtime supplies only the claimed Run and error text.
#[async_trait::async_trait]
pub trait AgentWorkerFailureApplicationPort: Send + Sync {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError>;

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError>;
}

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
