use super::super::{AgentApplication, AgentWorkerRun};
use crate::agent::agent_db_error_to_app;
use a4n_agent_runtime_contract::AgentWorkerActivationApplicationPort;
use a4n_db::agent;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerRunLeaseRequest;

impl AgentApplication {
    pub(crate) async fn mark_turn_active(&self, claimed: &AgentWorkerRun) -> Result<(), AppError> {
        agent::mark_turn_active_for_run_lease(
            self.connection(),
            claimed.run.id,
            &claimed.lease_token,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn mark_turn_active_request(
        &self,
        request: &AgentWorkerRunLeaseRequest,
    ) -> Result<(), AppError> {
        self.validate_worker_lease_request(request).await?;
        agent::mark_turn_active_for_run_lease(
            self.connection(),
            request.run_id,
            &request.lease.fence_token,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }
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
