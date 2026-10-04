//! Worker waiting state transitions.

use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use crate::agent::agent_db_error_to_app;
use a4n_agent_runtime_contract::AgentWorkerWaitApplicationPort;
use a4n_db::agent;
use a4n_error::AppError;
use a4n_worker_contract::{AgentWorkerRunLeaseRequest, AgentWorkerRunWaitRequest};

impl AgentApplication {
    pub(crate) async fn wait_run(
        &self,
        claimed: &AgentWorkerRun,
        reason: &str,
    ) -> Result<(), AppError> {
        validate_waiting_reason(reason)?;
        agent::wait_run_for_lease(
            self.executor(),
            claimed.run.id,
            &claimed.lease_token,
            reason,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn wait_run_request(
        &self,
        request: &AgentWorkerRunWaitRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        validate_waiting_reason(&request.reason)?;
        agent::wait_run_for_lease(
            self.executor(),
            request.run_id,
            &request.lease.fence_token,
            &request.reason,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }
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
