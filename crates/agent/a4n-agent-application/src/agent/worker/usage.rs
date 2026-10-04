use super::super::{AgentApplication, WorkerUsage};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerUsageApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerUsageRequest;

impl AgentApplication {
    pub(crate) async fn update_run_usage(
        &self,
        input: WorkerUsage,
    ) -> Result<agent::AgentRunRecord, AppError> {
        agent::update_run_usage(
            self.executor(),
            input.run_id,
            &input.lease_token,
            input.usage_json,
        )
        .await
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn update_run_usage_request(
        &self,
        request: &AgentWorkerUsageRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        agent::update_run_usage(
            self.executor(),
            request.run_id,
            &request.lease.fence_token,
            request.usage_json.clone(),
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }
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
