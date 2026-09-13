use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;

/// Runtime-facing boundary for marking a claimed Agent Turn active.
///
/// The lease-fenced lifecycle transition remains in the Agent owner; Runtime
/// only supplies the claimed Run.
#[async_trait::async_trait]
pub trait AgentWorkerActivationApplicationPort: Send + Sync {
    async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError>;

    async fn mark_turn_active_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError>;
}

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
