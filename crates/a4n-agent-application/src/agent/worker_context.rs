use super::{AgentApplication, AgentRunContext, AgentWorkerRun};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerContextSummaryRequest;
use serde_json::Value;

/// Runtime-facing boundary for loading one lease-validated Agent Run context.
///
/// ORM reads, scope normalization and lineage assembly remain in the Agent
/// owner; Runtime only receives the durable context snapshot.
#[async_trait::async_trait]
pub trait AgentWorkerContextApplicationPort: Send + Sync {
    async fn load_run_context(&self, claimed: &AgentWorkerRun)
    -> Result<AgentRunContext, AppError>;

    async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError>;

    async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerContextApplicationPort for AgentApplication {
    async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        AgentApplication::load_run_context(self, claimed).await
    }

    async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        AgentApplication::update_context_summary(self, claimed, context_summary).await
    }

    async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        AgentApplication::update_context_summary_request(self, request).await
    }
}
