//! Worker context snapshot and summary persistence.

use super::super::AgentRunContext;
use super::super::{AgentApplication, AgentWorkerRun};
use super::*;
use a4n_agent_runtime_contract::AgentWorkerContextApplicationPort;
use a4n_error::AppError;
use a4n_worker_contract::AgentWorkerContextSummaryRequest;
use serde_json::Value;

impl AgentApplication {
    /// Loads the complete durable context required for one model decision.
    pub(crate) async fn load_run_context(
        &self,
        claimed: &AgentWorkerRun,
    ) -> Result<AgentRunContext, AppError> {
        let run_row = agent::run_by_id(self.executor(), claimed.run.id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent run 不存在: {}", claimed.run.id)))?;
        if run_row.thread_id != claimed.run.thread_id
            || run_row.turn_id != claimed.run.turn_id
            || run_row.attempt != claimed.run.attempt
            || run_row.worker_id != claimed.run.worker_id
        {
            return Err(AppError::InvalidState {
                current: format!("Agent run {} 的 worker 快照已过期", claimed.run.id),
                operation: "加载 Agent worker context".into(),
            });
        }
        agent::validate_live_run_lease_record(
            &run_row,
            &claimed.lease_token,
            chrono::Utc::now().naive_utc(),
        )
        .map_err(|error| match error {
            AgentDbError::Conflict(message) => AppError::InvalidState {
                current: message,
                operation: "加载 Agent worker context".into(),
            },
            other => agent_db_error_to_app(other),
        })?;
        let thread_row = agent::thread_by_id(self.executor(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| {
                AppError::NotFound(format!("Agent thread 不存在: {}", run_row.thread_id))
            })?;
        let turn_row = agent::turn_by_id(self.executor(), run_row.turn_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
        validate_model_override_json(turn_row.model_override_json.as_ref())?;
        let thread = thread_snapshot(thread_row.clone())?;
        let covered_message_id = thread
            .context_summary
            .as_ref()
            .and_then(|summary| summary.get("covered_through_sequence"))
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let messages = agent::messages_for_context(
            self.executor(),
            run_row.thread_id,
            run_row.id,
            covered_message_id,
        )
        .await
        .map_err(agent_db_error_to_app)?;
        let mut scope = effective_agent_scope(
            &thread_row.profile_id,
            from_json(thread_row.scope_json.clone(), "agent_thread.scope_json")?,
        )?;
        self.hydrate_scope_chapter_number(&mut scope).await?;
        let editor_source_value = messages
            .iter()
            .find(|message| message.id == turn_row.input_message_id)
            .and_then(|message| message.payload_json.as_ref())
            .and_then(|payload| payload.get("editor_source"))
            .cloned();
        let editor_source = match editor_source_value {
            Some(value) => {
                Some(parse_persisted_editor_source(self.executor(), value, &thread_row).await?)
            }
            None => None,
        };
        let delegations = agent::delegations_by_thread(self.executor(), run_row.thread_id)
            .await
            .map_err(agent_db_error_to_app)?;
        let executions = agent::executions_by_runs(self.executor(), vec![run_row.id])
            .await
            .map_err(agent_db_error_to_app)?;
        let runtime_snapshot_required = runtime_snapshot_required(&run_row.usage_json, run_row.id)?;
        let delegation_depth = self
            .delegation_depth_for_run(run_row.id, run_row.parent_run_id)
            .await?;
        Ok(AgentRunContext {
            owner_user_id: thread_row.owner_user_id,
            profile_id: thread_row.profile_id.clone(),
            default_model_config_id: thread_row.default_model_config_id,
            execution_mode: turn_row.execution_mode.clone(),
            goal: turn_row.goal.clone(),
            budget: run_row.budget_json.clone(),
            usage: run_row.usage_json.clone(),
            runtime_snapshot_required,
            delegation_depth,
            run: run_snapshot(run_row),
            thread,
            turn: turn_snapshot(turn_row),
            messages: messages.into_iter().map(message_snapshot).collect(),
            delegations: delegations.into_iter().map(delegation_snapshot).collect(),
            executions: executions.into_iter().map(execution_snapshot).collect(),
            editor_source,
            scope,
        })
    }

    pub(crate) async fn update_context_summary(
        &self,
        claimed: &AgentWorkerRun,
        context_summary: Value,
    ) -> Result<(), AppError> {
        serde_json::from_value::<a4n_agent_runtime_contract::AgentContextSummary>(
            context_summary.clone(),
        )?
        .validate()?;
        agent::update_context_summary_for_run_lease(
            self.executor(),
            claimed.run.id,
            &claimed.lease_token,
            context_summary,
        )
        .await
        .map(|_| ())
        .map_err(agent_db_error_to_app)
    }

    pub(crate) async fn update_context_summary_request(
        &self,
        request: &AgentWorkerContextSummaryRequest,
    ) -> Result<(), AppError> {
        request
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let claimed = self
            .worker_run_for_lease_request(&AgentWorkerRunLeaseRequest {
                run_id: request.run_id,
                lease: request.lease.clone(),
            })
            .await?;
        self.update_context_summary(&claimed, request.context_summary.clone())
            .await
    }
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
