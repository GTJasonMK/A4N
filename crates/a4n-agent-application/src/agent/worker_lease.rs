use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;

/// Runtime-facing boundary for renewing the lease held by one Agent worker.
///
/// Claiming, expiry recovery and terminal state transitions remain separate
/// owner operations; this port only keeps the in-flight lease alive.
#[async_trait::async_trait]
pub trait AgentWorkerLeaseApplicationPort: Send + Sync {
    async fn renew_worker_lease(&self, claimed: &AgentWorkerRun) -> Result<(), AppError>;

    async fn renew_worker_lease_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError>;
}

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
