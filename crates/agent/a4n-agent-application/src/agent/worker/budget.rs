use super::super::{AgentApplication, AgentWorkerBudgetSnapshot};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerBudgetApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerBudgetRequest;

impl AgentApplication {
    pub(crate) async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.validate_worker_lease_request(&AgentWorkerRunLeaseRequest {
            run_id: request.run_id,
            lease: request.lease.clone(),
        })
        .await?;
        self.worker_budget_snapshot(request.run_id).await
    }

    pub(crate) async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        let snapshot = self.run_budget_snapshot(run_id).await?;
        Ok(AgentWorkerBudgetSnapshot {
            usage: snapshot.usage,
            max_tokens: snapshot.remaining.max_tokens,
            max_duration_seconds: snapshot.remaining.max_duration_seconds,
        })
    }
}

#[async_trait::async_trait]
impl AgentWorkerBudgetApplicationPort for AgentApplication {
    async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        AgentApplication::worker_budget_snapshot(self, run_id).await
    }

    async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError> {
        AgentApplication::worker_budget_snapshot_request(self, request).await
    }
}
