//! Persistence primitives for the shared autonomous agent runtime.
//!
//! This module intentionally contains no model/provider orchestration. It
//! owns the durable inbox, lease, event-sequence and idempotency boundaries;
//! the orchestrator decides what each record means.

use a4n_db_models::models::agent::{
    approval, delegation, event, execution, message, run, thread, turn,
};
use a4n_db_models::models::ai::system_settings;
use a4n_domain::agent::{
    AgentApprovalInput, AgentDelegationInput, AgentEventInput, AgentExecutionInput,
    AgentInboxInput, AgentMemoryInput, AgentMessageInput, AgentThreadCreateInput, AgentTurnInput,
};
use chrono::{Duration, NaiveDateTime, Utc};
use sea_orm::sea_query::{LockBehavior, LockType, OnConflict};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;

mod approvals;
pub use approvals::{
    expire_pending_approvals, request_approval_for_run_lease, resolve_approval_for_run,
};
mod delegations;
pub use delegations::{
    bind_delegation_child, complete_child_delegation, create_child_delegation, create_delegation,
    create_user_delegation, fail_child_delegation, update_delegation,
};
mod executions;
pub(crate) use executions::create_execution_on;
pub use executions::{create_execution_for_run_lease, update_execution_for_run_lease};
mod inbox;
pub use inbox::{
    claim_inbox, consume_inbox, consume_inbox_with_message, enqueue_inbox, release_inbox_claim,
    supersede_inbox,
};
mod memory;
pub use memory::{insert_memory, list_memory};
mod messages;
pub use messages::{
    append_message, append_message_for_run_lease,
    complete_assistant_streaming_message_for_run_lease,
    interrupt_assistant_streaming_message_for_run_lease, update_context_summary_for_run_lease,
    upsert_assistant_streaming_message_for_run_lease,
};
pub(crate) use messages::{
    append_message_event_on, append_message_on, append_parent_delegation_tool_message_on,
    finish_parent_delegation_execution_on, interrupt_streaming_messages_on,
    recover_child_delegation_on,
};
mod queries;
pub use queries::{
    active_runs_by_thread, approval_by_id, approvals_by_thread, create_thread,
    default_thread_by_owner, delegation_by_id, delegation_by_parent_and_key, delegations_by_thread,
    executions_by_runs, inbox_by_thread, list_threads_by_owner, message_by_id, messages_before,
    messages_by_run, messages_by_thread, messages_for_context, run_by_id, runs_by_thread,
    thread_by_canonical_binding, thread_by_id, turns_by_thread,
};
mod recovery;
pub use recovery::{cancel_run_tree, expired_running_runs, pause_run_tree, recover_expired_run};
mod runs;
pub use runs::{
    claim_editor_execution, claim_runnable_run, expire_editor_requests, fail_run_for_lease,
    finish_run_with_message, mark_turn_active_for_run_lease, recovery_required_for_lease,
    renew_run_lease, update_run_state, update_run_state_with_budget, update_run_usage,
    wait_run_for_lease, wait_run_for_lease_with_request, wake_waiting_run_for_reason,
};
pub(crate) use runs::{
    complete_turn, consume_observed_user_inbox_on, lock_delegations_on, lock_runs_on,
    lock_threads_on, lock_turns_on, refresh_thread_status_on, requeue_for_new_input_on,
    update_run_state_on,
};
mod turns_events;
pub use turns_events::{
    append_event, append_event_for_run_lease, create_turn_with_run, events_after,
    latest_events_by_thread,
};
pub(crate) use turns_events::{append_event_on, append_run_event_on, create_turn_with_run_on};
mod review;
pub use review::review_delegation;
mod completion;
pub use completion::{completion_blockers, request_user_input};
mod task_wait;
pub use task_wait::{external_task_wait, list_external_task_waits, wake_external_task_wait};
mod budget;
pub use budget::{AgentBudgetSnapshot, resume_run, run_budget_snapshot};
mod generation;
pub use generation::{
    AgentOwnedTask, AgentTaskExecutionInput, active_owned_generation_tasks, create_task_execution,
    generation_owner, generation_owner_stopped, update_task_execution,
};
mod workflow;
pub use workflow::{active_owned_workflows, bind_owned_workflow, workflow_owner};

pub const RUN_LEASE_SECONDS: i64 = 30;
pub const INBOX_LEASE_SECONDS: i64 = 30;
pub const EDITOR_RESPONSE_TTL_SECONDS: i64 = 30;
const AGENT_SKILL_PROFILE_KEY_PREFIX: &str = "agent_skill_profile:";

#[derive(Debug, thiserror::Error)]
pub enum AgentDbError {
    #[error("Agent 尚有未完成事项: {0:?}")]
    CompletionBlocked(Vec<a4n_domain::agent::AgentCompletionBlocker>),
    #[error("Agent 共享预算已耗尽: {0}")]
    BudgetExhausted(String),
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

#[derive(Debug, Clone)]
pub struct AgentChildDelegationInput {
    pub delegation: AgentDelegationInput,
    pub child_turn: AgentTurnInput,
    pub parent_lease_token: Option<String>,
}

/// User-created delegations intentionally do not carry a worker lease.  The
/// caller must hold the owning user's thread permission; this transaction
/// still locks the parent/child threads and parent run before changing the
/// durable run tree.
#[derive(Debug, Clone)]
pub struct AgentUserDelegationInput {
    pub delegation: AgentDelegationInput,
    pub child_turn: AgentTurnInput,
}

#[derive(Debug, Clone)]
pub struct AgentChildCompletionInput {
    pub child_run_id: i64,
    pub child_lease_token: String,
    pub final_message: AgentMessageInput,
    pub result_refs_json: Option<Value>,
    pub result_summary: String,
    pub observed_input_message_id: i64,
}

pub struct AgentQuestionInput {
    pub run_id: i64,
    pub lease_token: String,
    pub execution_id: i64,
    pub question: String,
    pub observation: Value,
}

#[derive(Debug, Clone)]
pub struct AgentChildFailureInput {
    pub child_run_id: i64,
    pub child_lease_token: String,
    pub error_message: String,
}

fn now() -> NaiveDateTime {
    Utc::now().naive_utc()
}

fn is_run_terminal(status: &str) -> bool {
    matches!(
        status,
        "completed" | "failed" | "cancelled" | "interrupted" | "recovery_required"
    )
}

/// Run status is the durable lifecycle authority.  `is_active` is retained as
/// an index/lease optimization, but older rows may have a stale value after a
/// process interruption.
pub fn is_run_active(status: &str) -> bool {
    !is_run_terminal(status)
}

fn agent_skill_profile_key(owner_user_id: i64, capability: &str) -> String {
    format!("{AGENT_SKILL_PROFILE_KEY_PREFIX}{owner_user_id}:{capability}")
}

pub async fn agent_skill_profile_ids<C: ConnectionTrait>(
    db: &C,
    owner_user_id: i64,
    capability: &str,
) -> Result<Option<Vec<String>>, AgentDbError> {
    let key = agent_skill_profile_key(owner_user_id, capability);
    let Some(setting) = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(key))
        .one(db)
        .await?
    else {
        return Ok(None);
    };
    serde_json::from_value(setting.value)
        .map(Some)
        .map_err(|error| {
            AgentDbError::InvalidArgument(format!("Agent Skill 配置解析失败: {error}"))
        })
}

pub async fn save_agent_skill_profile_ids(
    db: &DatabaseConnection,
    owner_user_id: i64,
    capability: &str,
    enabled_skill_ids: &[String],
) -> Result<(), AgentDbError> {
    let key = agent_skill_profile_key(owner_user_id, capability);
    let value = serde_json::to_value(enabled_skill_ids).map_err(|error| {
        AgentDbError::InvalidArgument(format!("Agent Skill 配置序列化失败: {error}"))
    })?;
    let now = now();
    let existing = system_settings::Entity::find()
        .filter(system_settings::Column::Key.eq(&key))
        .one(db)
        .await?;
    if let Some(setting) = existing {
        let mut active: system_settings::ActiveModel = setting.into();
        active.value = Set(value);
        active.category = Set("agent_skill".into());
        active.description = Set(Some("Agent capability 的启用 Skill".into()));
        active.updated_at = Set(now);
        active.update(db).await?;
    } else {
        system_settings::ActiveModel {
            key: Set(key),
            value: Set(value),
            category: Set("agent_skill".into()),
            description: Set(Some("Agent capability 的启用 Skill".into())),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?;
    }
    Ok(())
}

pub fn validate_live_run_lease(
    row: &run::Model,
    lease_token: &str,
    timestamp: NaiveDateTime,
) -> Result<(), AgentDbError> {
    if lease_token.trim().is_empty()
        || row.lease_token.as_deref() != Some(lease_token)
        || !row.is_active
        || row.status != "running"
        || row
            .lease_expires_at
            .is_none_or(|expires_at| expires_at <= timestamp)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent run lease 已失效: {}",
            row.id
        )));
    }
    Ok(())
}

fn is_turn_terminal(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled")
}

fn validate_message_status(status: &str) -> Result<(), AgentDbError> {
    if matches!(status, "streaming" | "complete" | "interrupted") {
        Ok(())
    } else {
        Err(AgentDbError::InvalidArgument(format!(
            "未知 Agent message 状态: {status}"
        )))
    }
}

fn validate_turn_status(status: &str) -> Result<(), AgentDbError> {
    if matches!(
        status,
        "queued" | "active" | "completed" | "failed" | "cancelled"
    ) {
        Ok(())
    } else {
        Err(AgentDbError::InvalidArgument(format!(
            "未知 Agent turn 状态: {status}"
        )))
    }
}

fn validate_execution_status(status: &str) -> Result<(), AgentDbError> {
    if matches!(
        status,
        "prepared"
            | "awaiting_user"
            | "dispatching"
            | "running"
            | "succeeded"
            | "failed"
            | "rejected"
            | "cancelled"
            | "outcome_unknown"
    ) {
        Ok(())
    } else {
        Err(AgentDbError::InvalidArgument(format!(
            "未知 Agent execution 状态: {status}"
        )))
    }
}

fn is_execution_terminal(status: &str) -> bool {
    matches!(
        status,
        "succeeded" | "failed" | "rejected" | "cancelled" | "outcome_unknown"
    )
}

#[derive(Clone, Copy)]
enum ExecutionTermination {
    Failed,
    Cancelled,
    RecoveryRequired,
    Paused,
}

/// Closes every non-terminal execution owned by a run while the run's
/// lifecycle lock is held.  A dispatching/running write may already have
/// reached its side-effect boundary, so it is deliberately projected as
/// `outcome_unknown`; model and read executions can be safely marked failed.
async fn terminate_executions_on<C>(
    db: &C,
    run_ids: impl IntoIterator<Item = i64>,
    termination: ExecutionTermination,
    error_message: Option<&str>,
    timestamp: NaiveDateTime,
) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let run_ids = run_ids.into_iter().collect::<Vec<_>>();
    if run_ids.is_empty() {
        return Ok(());
    }
    let rows = execution::Entity::find()
        .filter(execution::Column::RunId.is_in(run_ids))
        .filter(execution::Column::Status.is_in([
            "prepared",
            "awaiting_user",
            "dispatching",
            "running",
        ]))
        .all(db)
        .await?;
    for row in rows {
        let status = match termination {
            ExecutionTermination::Paused => match row.status.as_str() {
                "dispatching" | "running" if row.execution_kind == "model_call" => "failed",
                "dispatching" | "running"
                    if row.effect.as_deref() != Some("read")
                        && !matches!(row.replay_policy.as_deref(), Some("safe" | "idempotent")) =>
                {
                    "outcome_unknown"
                }
                _ => continue,
            },
            ExecutionTermination::Cancelled => match row.status.as_str() {
                "prepared" | "awaiting_user" => "cancelled",
                "dispatching" | "running"
                    if row.execution_kind == "model_call"
                        || row.effect.as_deref() == Some("read") =>
                {
                    "cancelled"
                }
                "dispatching" | "running" => "outcome_unknown",
                _ => continue,
            },
            ExecutionTermination::Failed | ExecutionTermination::RecoveryRequired => {
                match row.status.as_str() {
                    "prepared" | "awaiting_user" => "failed",
                    "dispatching" | "running"
                        if row.execution_kind == "model_call"
                            || row.effect.as_deref() == Some("read") =>
                    {
                        "failed"
                    }
                    "dispatching" | "running" => "outcome_unknown",
                    _ => continue,
                }
            }
        };
        let mut active: execution::ActiveModel = row.into();
        active.status = Set(status.to_string());
        active.error_message = Set(error_message.map(str::to_string));
        active.updated_at = Set(timestamp);
        active.completed_at = Set(Some(timestamp));
        active.update(db).await?;
    }
    Ok(())
}

fn validate_run_status(status: &str) -> Result<(), AgentDbError> {
    if matches!(
        status,
        "queued"
            | "running"
            | "waiting"
            | "pausing"
            | "paused"
            | "cancelling"
            | "cancelled"
            | "completed"
            | "failed"
            | "interrupted"
            | "recovery_required"
    ) {
        Ok(())
    } else {
        Err(AgentDbError::InvalidArgument(format!(
            "未知 Agent run 状态: {status}"
        )))
    }
}

fn is_delegation_terminal(status: &str) -> bool {
    matches!(status, "accepted" | "failed" | "cancelled" | "interrupted")
}

fn validate_delegation_status(status: &str) -> Result<(), AgentDbError> {
    if matches!(
        status,
        "pending"
            | "running"
            | "awaiting_acceptance"
            | "revision_requested"
            | "accepted"
            | "failed"
            | "cancelled"
            | "interrupted"
    ) {
        Ok(())
    } else {
        Err(AgentDbError::InvalidArgument(format!(
            "未知 Agent delegation 状态: {status}"
        )))
    }
}

fn projected_thread_status(active_run: Option<&run::Model>) -> Result<&'static str, AgentDbError> {
    match active_run.map(|row| row.status.as_str()) {
        None => Ok("idle"),
        Some("queued" | "running") => Ok("running"),
        Some("waiting") => Ok("waiting"),
        Some("pausing") => Ok("pausing"),
        Some("paused") => Ok("paused"),
        Some("cancelling") => Ok("cancelling"),
        Some(status) => Err(AgentDbError::Conflict(format!(
            "Agent thread 存在状态不可投影的 active run: {status}"
        ))),
    }
}

pub fn sha256_hex(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn json_sha256<T>(value: &T) -> Result<String, AgentDbError>
where
    T: Serialize,
{
    let bytes = serde_json_canonicalizer::to_vec(value)
        .map_err(|error| AgentDbError::InvalidArgument(format!("无法规范化 JSON: {error}")))?;
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    Ok(format!("{:x}", hasher.finalize()))
}
