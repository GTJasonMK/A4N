use super::AgentApplication;
use a4n_error::AppError;
use a4n_worker_contract::{AgentWorkerBudgetRequest, AgentWorkerBudgetSnapshotWire};
use serde_json::Value;

/// Runtime-facing boundary for reading the budget facts needed by the Agent
/// worker. Budget ownership remains in `AgentApplication`.
#[async_trait::async_trait]
pub trait AgentWorkerBudgetApplicationPort: Send + Sync {
    async fn worker_budget_snapshot(
        &self,
        run_id: i64,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError>;

    async fn worker_budget_snapshot_request(
        &self,
        request: &AgentWorkerBudgetRequest,
    ) -> Result<AgentWorkerBudgetSnapshot, AppError>;
}

/// Minimal projection needed by the Agent worker budget checks.
#[derive(Debug, Clone)]
pub struct AgentWorkerBudgetSnapshot {
    pub usage: Value,
    pub max_tokens: Option<u64>,
    pub max_duration_seconds: Option<u64>,
}

impl From<AgentWorkerBudgetSnapshot> for AgentWorkerBudgetSnapshotWire {
    fn from(snapshot: AgentWorkerBudgetSnapshot) -> Self {
        Self {
            usage: snapshot.usage,
            max_tokens: snapshot.max_tokens,
            max_duration_seconds: snapshot.max_duration_seconds,
        }
    }
}

impl From<AgentWorkerBudgetSnapshotWire> for AgentWorkerBudgetSnapshot {
    fn from(snapshot: AgentWorkerBudgetSnapshotWire) -> Self {
        Self {
            usage: snapshot.usage,
            max_tokens: snapshot.max_tokens,
            max_duration_seconds: snapshot.max_duration_seconds,
        }
    }
}

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
