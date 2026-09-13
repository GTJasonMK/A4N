use super::{AgentApplication, WorkerUsage};
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerUsageRequest;

/// Runtime-facing boundary for persisting worker usage counters.
#[async_trait::async_trait]
pub trait AgentWorkerUsageApplicationPort: Send + Sync {
    async fn update_run_usage(&self, input: WorkerUsage) -> Result<(), AppError>;

    async fn update_run_usage_request(
        &self,
        request: &AgentWorkerUsageRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
impl AgentWorkerUsageApplicationPort for AgentApplication {
    async fn update_run_usage(&self, input: WorkerUsage) -> Result<(), AppError> {
        AgentApplication::update_run_usage(self, input)
            .await
            .map(|_| ())
    }

    async fn update_run_usage_request(
        &self,
        request: &AgentWorkerUsageRequest,
    ) -> Result<(), AppError> {
        AgentApplication::update_run_usage_request(self, request).await
    }
}
