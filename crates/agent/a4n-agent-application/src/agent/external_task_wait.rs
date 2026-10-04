use super::AgentApplication;
use crate::agent::{agent_db_error_to_app, from_json};
use a4n_agent_runtime_contract::{
    AgentExternalTaskWaitApplicationPort, AgentExternalTaskWaitSnapshot,
};
use a4n_db::agent;
use a4n_error::AppError;

impl AgentApplication {
    pub async fn external_task_wait(
        &self,
        run_id: i64,
        execution_id: i64,
    ) -> Result<Option<AgentExternalTaskWaitSnapshot>, AppError> {
        let Some(wait) = agent::external_task_wait(self.executor(), run_id, execution_id)
            .await
            .map_err(agent_db_error_to_app)?
        else {
            return Ok(None);
        };
        let duration_exhausted = agent::run_budget_snapshot(self.executor(), wait.run.id)
            .await
            .map_err(agent_db_error_to_app)?
            .remaining
            .max_duration_seconds
            == Some(0);
        Ok(Some(AgentExternalTaskWaitSnapshot {
            run_id: wait.run.id,
            owner_user_id: wait.thread.owner_user_id,
            scope: from_json(wait.thread.scope_json, "agent_thread.scope_json")?,
            execution_id: wait.execution.id,
            tool_name: wait.execution.tool_name,
            arguments_json: wait.execution.arguments_json,
            duration_exhausted,
        }))
    }

    pub async fn external_task_waits(
        &self,
        after_run_id: i64,
    ) -> Result<Vec<AgentExternalTaskWaitSnapshot>, AppError> {
        let rows = agent::list_external_task_waits(self.executor(), after_run_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let mut waits = Vec::with_capacity(rows.len());
        for wait in rows {
            let duration_exhausted = agent::run_budget_snapshot(self.executor(), wait.run.id)
                .await
                .map_err(agent_db_error_to_app)?
                .remaining
                .max_duration_seconds
                == Some(0);
            waits.push(AgentExternalTaskWaitSnapshot {
                run_id: wait.run.id,
                owner_user_id: wait.thread.owner_user_id,
                scope: from_json(wait.thread.scope_json, "agent_thread.scope_json")?,
                execution_id: wait.execution.id,
                tool_name: wait.execution.tool_name,
                arguments_json: wait.execution.arguments_json,
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
        agent::wake_external_task_wait(self.executor(), run_id, execution_id)
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

    async fn external_task_wait(
        &self,
        run_id: i64,
        execution_id: i64,
    ) -> Result<Option<AgentExternalTaskWaitSnapshot>, AppError> {
        AgentApplication::external_task_wait(self, run_id, execution_id).await
    }

    async fn wake_external_task_wait(
        &self,
        run_id: i64,
        execution_id: i64,
    ) -> Result<(), AppError> {
        AgentApplication::wake_external_task_wait(self, run_id, execution_id).await
    }
}
