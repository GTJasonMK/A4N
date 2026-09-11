use super::{AgentApplication, AgentRunContext, AgentWorkerRun};
use a4n_error::AppError;

/// Runtime-facing boundary for loading one lease-validated Agent Run context.
///
/// ORM reads, scope normalization and lineage assembly remain in the Agent
/// owner; Runtime only receives the durable context snapshot.
#[async_trait::async_trait]
pub trait AgentWorkerContextApplicationPort: Send + Sync {
    async fn load_run_context(&self, claimed: &AgentWorkerRun)
    -> Result<AgentRunContext, AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerContextApplicationPort for AgentApplication {
    async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        AgentApplication::load_run_context(self, claimed).await
    }
}
