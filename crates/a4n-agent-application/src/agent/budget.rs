use super::{AgentApplication, AgentWorkerBudgetSnapshot};
use a4n_agent_runtime_contract::AgentWorkerBudgetApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerBudgetRequest;

impl AgentApplication {
    pub async fn worker_budget_snapshot(
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
