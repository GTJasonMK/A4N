use super::AgentApplication;
use crate::agent::{agent_db_error_to_app, from_json};
use a4n_api_contract::agent::AgentScope;
use a4n_db::agent;
use a4n_error::AppError;
use serde_json::Value;

/// Runtime-facing boundary for reconciling waits on external task results.
///
/// Cancellation, editor waits and lease transitions intentionally stay on the
/// durable worker façade; this port only exposes the scan and wake operations
/// needed by the external-result reconciler.
#[async_trait::async_trait]
pub trait AgentExternalTaskWaitApplicationPort: Send + Sync {
    async fn external_task_waits(
        &self,
        after_run_id: i64,
    ) -> Result<Vec<AgentExternalTaskWaitSnapshot>, AppError>;

    async fn wake_external_task_wait(&self, run_id: i64, execution_id: i64)
    -> Result<(), AppError>;
}

/// Minimal projection needed to reconcile a wait on an external task.
#[derive(Debug, Clone)]
pub struct AgentExternalTaskWaitSnapshot {
    pub run_id: i64,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub execution_id: i64,
    pub tool_name: Option<String>,
    pub arguments_json: Option<Value>,
    pub duration_exhausted: bool,
}

impl AgentApplication {
    pub async fn external_task_waits(
        &self,
        after_run_id: i64,
    ) -> Result<Vec<AgentExternalTaskWaitSnapshot>, AppError> {
        let rows = agent::list_external_task_waits(self.connection(), after_run_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut waits = Vec::with_capacity(rows.len());
        for (run, thread, execution) in rows {
            let duration_exhausted = agent::run_budget_snapshot(self.connection(), run.id)
                .await
                .map_err(agent_db_error_to_app)?
                .remaining
                .max_duration_seconds
                == Some(0);
            waits.push(AgentExternalTaskWaitSnapshot {
                run_id: run.id,
                owner_user_id: thread.owner_user_id,
                scope: from_json(thread.scope_json, "agent_thread.scope_json")?,
                execution_id: execution.id,
                tool_name: execution.tool_name,
                arguments_json: execution.arguments_json,
                duration_exhausted,
            });
        }
        Ok(waits)
    }

    pub async fn wake_external_task_wait(
        &self,
        run_id: i64,
        execution_id: i64,
    ) -> Result<(), AppError> {
        agent::wake_external_task_wait(self.connection(), run_id, execution_id)
            .await
            .map_err(agent_db_error_to_app)
    }
}

#[async_trait::async_trait]
impl AgentExternalTaskWaitApplicationPort for AgentApplication {
    async fn external_task_waits(
        &self,
        after_run_id: i64,
    ) -> Result<Vec<AgentExternalTaskWaitSnapshot>, AppError> {
        AgentApplication::external_task_waits(self, after_run_id).await
    }

    async fn wake_external_task_wait(
        &self,
        run_id: i64,
        execution_id: i64,
    ) -> Result<(), AppError> {
        AgentApplication::wake_external_task_wait(self, run_id, execution_id).await
    }
}
