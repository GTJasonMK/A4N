use super::{AgentApplication, AgentWorkerRun};
use a4n_error::AppError;

/// Runtime-facing boundary for claiming one queued Agent worker Run.
///
/// Queue eligibility, lease creation and thread lookup stay in the Agent
/// owner; Runtime receives only the worker snapshot it needs to execute.
#[async_trait::async_trait]
pub trait AgentWorkerClaimApplicationPort: Send + Sync {
    async fn claim_worker_run(&self, worker_id: &str) -> Result<Option<AgentWorkerRun>, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerClaimApplicationPort for AgentApplication {
    async fn claim_worker_run(&self, worker_id: &str) -> Result<Option<AgentWorkerRun>, AppError> {
        AgentApplication::claim_worker_run(self, worker_id).await
    }
}
