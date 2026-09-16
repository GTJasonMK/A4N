//! Persistence primitives for the shared autonomous agent runtime.
//!
//! This module intentionally contains no model/provider orchestration. It
//! owns the durable inbox, lease, event-sequence and idempotency boundaries;
//! the orchestrator decides what each record means.

use a4n_db_models::models::agent::{
    approval, delegation, event, execution, inbox, memory, message, run, thread, turn,
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

mod review;
pub use review::review_delegation;
mod task_wait;
pub use task_wait::{external_task_wait, list_external_task_waits, wake_external_task_wait};
mod budget;
pub use budget::{AgentBudgetSnapshot, run_budget_snapshot};
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
            ExecutionTermination::Cancelled => match row.status.as_str() {
                "prepared" | "awaiting_user" => "cancelled",
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

pub async fn create_thread<C>(
    db: &C,
    input: AgentThreadCreateInput,
) -> Result<thread::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if input.owner_user_id <= 0 {
        return Err(AgentDbError::InvalidArgument("owner_user_id 无效".into()));
    }
    if input.profile_id.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument("profile_id 不能为空".into()));
    }
    let timestamp = now();
    thread::ActiveModel {
        owner_user_id: Set(input.owner_user_id),
        thread_kind: Set(input.thread_kind),
        profile_id: Set(input.profile_id),
        title: Set(input.title),
        project_id: Set(input.project_id),
        resource_type: Set(input.resource_type),
        resource_id: Set(input.resource_id),
        canonical_binding_key: Set(input.canonical_binding_key),
        scope_json: Set(input.scope_json),
        default_model_config_id: Set(input.default_model_config_id),
        status: Set("idle".to_string()),
        is_default: Set(input.is_default),
        revision: Set(1),
        next_turn_sequence: Set(0),
        next_event_sequence: Set(0),
        context_summary: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        archived_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::Database)
}

pub async fn thread_by_id<C>(db: &C, id: i64) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn list_threads_by_owner<C>(
    db: &C,
    owner_user_id: i64,
) -> Result<Vec<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .order_by_desc(thread::Column::UpdatedAt)
        .order_by_desc(thread::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn default_thread_by_owner<C>(
    db: &C,
    owner_user_id: i64,
) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .filter(thread::Column::IsDefault.eq(true))
        .filter(thread::Column::ArchivedAt.is_null())
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn thread_by_canonical_binding<C>(
    db: &C,
    owner_user_id: i64,
    canonical_binding_key: &str,
) -> Result<Option<thread::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    thread::Entity::find()
        .filter(thread::Column::OwnerUserId.eq(owner_user_id))
        .filter(thread::Column::CanonicalBindingKey.eq(canonical_binding_key))
        .filter(thread::Column::ArchivedAt.is_null())
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn messages_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = message::Entity::find()
        .filter(message::Column::ThreadId.eq(thread_id))
        .order_by_desc(message::Column::CreatedAt)
        .order_by_desc(message::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn message_by_id<C>(db: &C, id: i64) -> Result<Option<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn messages_by_run<C>(db: &C, run_id: i64) -> Result<Vec<message::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    message::Entity::find()
        .filter(message::Column::RunId.eq(run_id))
        .order_by_asc(message::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn turns_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<turn::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = turn::Entity::find()
        .filter(turn::Column::ThreadId.eq(thread_id))
        .order_by_desc(turn::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn runs_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = run::Entity::find()
        .filter(run::Column::ThreadId.eq(thread_id))
        .order_by_desc(run::Column::CreatedAt)
        .order_by_desc(run::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn active_runs_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    Ok(run::Entity::find()
        .filter(run::Column::ThreadId.eq(thread_id))
        .order_by_desc(run::Column::UpdatedAt)
        .order_by_desc(run::Column::Id)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?
        .into_iter()
        .filter(|row| is_run_active(&row.status))
        .collect())
}

pub async fn run_by_id<C>(db: &C, id: i64) -> Result<Option<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    run::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn executions_by_runs<C>(
    db: &C,
    run_ids: Vec<i64>,
) -> Result<Vec<execution::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    if run_ids.is_empty() {
        return Ok(Vec::new());
    }
    execution::Entity::find()
        .filter(execution::Column::RunId.is_in(run_ids))
        .order_by_asc(execution::Column::RunId)
        .order_by_asc(execution::Column::Sequence)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn delegations_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(delegation::Column::ParentThreadId.eq(thread_id))
                .add(delegation::Column::ChildThreadId.eq(thread_id)),
        )
        .order_by_asc(delegation::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn delegation_by_id<C>(db: &C, id: i64) -> Result<Option<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn delegation_by_parent_and_key<C>(
    db: &C,
    parent_run_id: i64,
    delegation_key: &str,
) -> Result<Option<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    delegation::Entity::find()
        .filter(delegation::Column::ParentRunId.eq(parent_run_id))
        .filter(delegation::Column::DelegationKey.eq(delegation_key))
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn inbox_by_thread<C>(db: &C, thread_id: i64) -> Result<Vec<inbox::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    inbox::Entity::find()
        .filter(inbox::Column::ThreadId.eq(thread_id))
        .order_by_asc(inbox::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn approvals_by_thread<C>(
    db: &C,
    thread_id: i64,
) -> Result<Vec<approval::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    approval::Entity::find()
        .filter(approval::Column::ThreadId.eq(thread_id))
        .order_by_asc(approval::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn approval_by_id<C>(db: &C, id: i64) -> Result<Option<approval::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    approval::Entity::find_by_id(id)
        .one(db)
        .await
        .map_err(AgentDbError::Database)
}

async fn append_message_on<C>(
    db: &C,
    input: AgentMessageInput,
) -> Result<message::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_message_status(&input.status)?;
    // OpenAI-compatible tool-call turns commonly carry an empty assistant
    // content; the durable payload is the tool_calls array in that case.
    let tool_call_decision = input.role == "assistant"
        && input
            .payload_json
            .as_ref()
            .and_then(|payload| payload.get("tool_calls"))
            .and_then(Value::as_array)
            .is_some_and(|calls| !calls.is_empty());
    if input.content.trim().is_empty()
        && !tool_call_decision
        && !(input.role == "assistant" && input.status == "streaming")
    {
        return Err(AgentDbError::InvalidArgument("Agent 消息不能为空".into()));
    }
    let digest = sha256_hex(&input.content);
    if let Some(client_id) = input.client_message_id.as_deref()
        && let Some(existing) = message::Entity::find()
            .filter(message::Column::ThreadId.eq(input.thread_id))
            .filter(message::Column::ClientMessageId.eq(client_id))
            .one(db)
            .await?
    {
        if existing.content_sha256 != digest || existing.role != input.role {
            return Err(AgentDbError::Conflict(format!(
                "client_message_id 已用于不同消息: {}",
                client_id
            )));
        }
        return Ok(existing);
    }
    let timestamp = now();
    let active = message::ActiveModel {
        thread_id: Set(input.thread_id),
        turn_id: Set(input.turn_id),
        run_id: Set(input.run_id),
        role: Set(input.role),
        sender_kind: Set(input.sender_kind),
        sender_thread_id: Set(input.sender_thread_id),
        client_message_id: Set(input.client_message_id.clone()),
        content_sha256: Set(digest.clone()),
        content: Set(input.content),
        status: Set(input.status),
        payload_json: Set(input.payload_json),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        ..Default::default()
    };
    let Some(client_id) = input.client_message_id else {
        return active.insert(db).await.map_err(AgentDbError::Database);
    };
    message::Entity::insert(active)
        .on_conflict(
            OnConflict::columns([message::Column::ThreadId, message::Column::ClientMessageId])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let existing = message::Entity::find()
        .filter(message::Column::ThreadId.eq(input.thread_id))
        .filter(message::Column::ClientMessageId.eq(&client_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "消息插入或幂等冲突后无法重读: thread={}, client_message_id={client_id}",
                input.thread_id
            ))
        })?;
    if existing.content_sha256 != digest {
        return Err(AgentDbError::Conflict(format!(
            "client_message_id 已用于不同消息: {client_id}"
        )));
    }
    Ok(existing)
}

fn delegation_tool_call_id(execution: &execution::Model) -> Result<String, AgentDbError> {
    execution
        .execution_key
        .strip_prefix("tool-call:")
        .filter(|value| !value.trim().is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            AgentDbError::Conflict(format!(
                "Agent delegation execution 缺少可恢复的 tool_call_id: {}",
                execution.id
            ))
        })
}

async fn append_parent_delegation_tool_message_on<C>(
    db: &C,
    parent_run: &run::Model,
    child_run: &run::Model,
    delegation: &delegation::Model,
    parent_execution: &execution::Model,
    status: &str,
    result: Value,
) -> Result<message::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if parent_execution.run_id != parent_run.id
        || parent_execution.execution_kind != "delegation"
        || parent_execution.tool_name.as_deref() != Some("delegate")
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent delegation execution 关联不一致: {}",
            parent_execution.id
        )));
    }
    let tool_call_id = delegation_tool_call_id(parent_execution)?;
    let payload = json!({
        "execution_id": parent_execution.id.to_string(),
        "tool": "delegate",
        "tool_call_id": tool_call_id,
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "status": status,
        "result": result,
    });
    // Keep the durable tool message content identical to the execution's
    // `result_json`.  A resumed parent may replay the succeeded execution via
    // the normal tool-observation path; matching content makes that replay
    // idempotent instead of colliding on `agent-tool:{execution_id}`.
    let content = serde_json::to_string(&result).map_err(|error| {
        AgentDbError::InvalidArgument(format!("无法序列化 delegation result: {error}"))
    })?;
    append_message_on(
        db,
        AgentMessageInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            role: "tool".into(),
            sender_kind: "tool".into(),
            sender_thread_id: Some(child_run.thread_id),
            client_message_id: Some(format!("agent-tool:{}", parent_execution.id)),
            content,
            status: "complete".into(),
            payload_json: Some(payload),
        },
    )
    .await
}

async fn finish_parent_delegation_execution_on<C>(
    db: &C,
    parent_execution: execution::Model,
    status: &str,
    result_json: Value,
    result_summary: String,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
    timestamp: NaiveDateTime,
) -> Result<execution::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if is_execution_terminal(&parent_execution.status) {
        if parent_execution.status == status {
            return Ok(parent_execution);
        }
        return Err(AgentDbError::Conflict(format!(
            "Agent parent delegation execution 已终止，不能从 {} 变为 {}: {}",
            parent_execution.status, status, parent_execution.id
        )));
    }
    if !matches!(
        parent_execution.status.as_str(),
        "prepared" | "awaiting_user" | "dispatching" | "running"
    ) {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent delegation execution 不在可完成状态: {} ({})",
            parent_execution.id, parent_execution.status
        )));
    }
    let mut active: execution::ActiveModel = parent_execution.into();
    active.status = Set(status.to_string());
    active.result_json = Set(Some(result_json));
    active.result_summary = Set(Some(result_summary));
    active.result_refs_json = Set(result_refs_json);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.update(db).await.map_err(AgentDbError::Database)
}

/// Projects an expired child lease to its parent. The child run is already
/// transitioned to `recovery_required` by the caller; this helper closes the
/// delegation/execution, appends one durable tool observation, enqueues one
/// child-result inbox item and wakes a parent waiting on that child.
async fn recover_child_delegation_on<C>(
    db: &C,
    child_run: &run::Model,
    parent_run: &run::Model,
    error_message: Option<&str>,
    timestamp: NaiveDateTime,
) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let Some(mut delegation) = delegation::Entity::find()
        .filter(delegation::Column::ChildRunId.eq(child_run.id))
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Err(AgentDbError::Conflict(format!(
            "Agent child run 缺少 delegation，无法通知 parent: {}",
            child_run.id
        )));
    };
    if delegation.parent_run_id != parent_run.id
        || delegation.parent_thread_id != parent_run.thread_id
        || delegation.child_thread_id != child_run.thread_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child recovery delegation lineage 不一致: {}",
            delegation.id
        )));
    }
    if is_delegation_terminal(&delegation.status) {
        return Ok(());
    }
    let reason = error_message
        .unwrap_or("Agent child lease expired; explicit reconciliation required")
        .to_string();
    let mut delegation_active: delegation::ActiveModel = delegation.clone().into();
    delegation_active.status = Set("interrupted".into());
    delegation_active.error_message = Set(Some(reason.clone()));
    delegation_active.result_summary = Set(Some("Child Agent 进入 recovery_required".into()));
    delegation_active.updated_at = Set(timestamp);
    delegation_active.completed_at = Set(Some(timestamp));
    delegation = delegation_active.update(db).await?;

    append_run_event_on(
        db,
        child_run,
        "run_recovery_required",
        "recovery_required",
        "Child Agent 需要显式恢复或对账",
        None,
        Some(delegation.id),
        None,
        Some(json!({ "reason": reason.clone() })),
    )
    .await?;

    let mut inbox_payload = json!({
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "recovery_required": true,
        "reason": reason.clone(),
    });
    if let Some(execution_id) = delegation.parent_execution_id {
        let parent_execution = execution::Entity::find_by_id(execution_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!(
                    "Agent parent delegation execution 不存在: {execution_id}"
                ))
            })?;
        let result = json!({
            "delegation_id": delegation.id.to_string(),
            "child_run_id": child_run.id.to_string(),
            "recovery_required": true,
            "reason": reason.clone(),
        });
        let message = append_parent_delegation_tool_message_on(
            db,
            parent_run,
            child_run,
            &delegation,
            &parent_execution,
            "outcome_unknown",
            result.clone(),
        )
        .await?;
        finish_parent_delegation_execution_on(
            db,
            parent_execution,
            "outcome_unknown",
            result,
            "Child Agent 需要显式恢复或对账".into(),
            None,
            Some(reason.clone()),
            timestamp,
        )
        .await?;
        inbox_payload["message_id"] = json!(message.id.to_string());
    }
    enqueue_inbox(
        db,
        AgentInboxInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            source_kind: "subagent".into(),
            source_thread_id: Some(child_run.thread_id),
            inbox_kind: "child_result".into(),
            delivery_mode: "followup".into(),
            priority: 20,
            dedupe_key: format!("child-recovery:{}", delegation.id),
            content: format!("Child Agent recovery required: {reason}"),
            payload_json: Some(inbox_payload),
            available_at: None,
        },
    )
    .await?;
    append_event_on(
        db,
        AgentEventInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            execution_id: delegation.parent_execution_id,
            delegation_id: Some(delegation.id),
            message_id: None,
            actor_kind: "agent".into(),
            actor_id: Some(child_run.id.to_string()),
            kind: "child_recovery_required".into(),
            status: "interrupted".into(),
            public_text: "Child Agent 需要显式恢复或对账".into(),
            payload_json: Some(json!({ "reason": reason })),
        },
    )
    .await?;
    if parent_run.is_active
        && parent_run.status == "waiting"
        && parent_run.waiting_reason.as_deref() == Some("child")
    {
        let mut parent_active: run::ActiveModel = parent_run.clone().into();
        parent_active.status = Set("queued".into());
        parent_active.waiting_reason = Set(None);
        parent_active.worker_id = Set(None);
        parent_active.lease_token = Set(None);
        parent_active.lease_expires_at = Set(None);
        parent_active.next_attempt_at = Set(timestamp);
        parent_active.updated_at = Set(timestamp);
        parent_active.update(db).await?;
    }
    Ok(())
}

pub async fn append_message<C>(
    db: &C,
    input: AgentMessageInput,
) -> Result<message::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    append_message_on(db, input).await
}

/// Appends a message only while the caller still owns the Run lease.
///
/// Tool output is part of the durable execution trace; fencing it prevents a
/// stale worker from adding observations after recovery has transferred the
/// run to another owner.
pub async fn append_message_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    input: AgentMessageInput,
) -> Result<message::Model, AgentDbError> {
    let tx = db.begin().await?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    if input.run_id != Some(run_id) || input.thread_id != run_row.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent message 与 run 不一致: {run_id}"
        )));
    }
    let row = append_message_on(&tx, input).await?;
    append_message_event_on(&tx, &row, "message_appended", "complete").await?;
    tx.commit().await?;
    Ok(row)
}

/// Stores a context summary only while the worker still owns the Run lease.
/// The summary is durable thread state, so a recovered worker cannot overwrite
/// a newer summary from the next lease holder.
pub async fn update_context_summary_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    context_summary: Value,
) -> Result<thread::Model, AgentDbError> {
    if !context_summary.is_object() {
        return Err(AgentDbError::InvalidArgument(
            "Agent context_summary 必须是 JSON 对象".into(),
        ));
    }
    let serialized = serde_json::to_string(&context_summary).map_err(|error| {
        AgentDbError::InvalidArgument(format!("Agent context_summary 无效: {error}"))
    })?;
    if serialized.len() > 256 * 1024 {
        return Err(AgentDbError::InvalidArgument(
            "Agent context_summary 超过 256 KiB 限制".into(),
        ));
    }
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    let thread_row = thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    if run_row.thread_id != thread_row.id {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 与 thread 不一致: {run_id}"
        )));
    }
    let mut active: thread::ActiveModel = thread_row.into();
    active.context_summary = Set(Some(serialized));
    active.updated_at = Set(now());
    let updated = active.update(&tx).await?;
    tx.commit().await?;
    Ok(updated)
}

/// Upserts one assistant stream projection while the worker still owns its
/// Run lease. The client id is the stream identity, so retries update the
/// same message instead of appending a second assistant response.
pub async fn upsert_assistant_streaming_message_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    input: AgentMessageInput,
) -> Result<message::Model, AgentDbError> {
    if input.role != "assistant" || input.status != "streaming" || input.client_message_id.is_none()
    {
        return Err(AgentDbError::InvalidArgument(
            "assistant streaming message 必须带 client_message_id 且状态为 streaming".into(),
        ));
    }
    let client_message_id = input.client_message_id.clone().expect("checked above");
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    if input.thread_id != run_row.thread_id
        || input.turn_id != Some(run_row.turn_id)
        || input.run_id != Some(run_id)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent streaming message 与 run 不一致: {run_id}"
        )));
    }
    let timestamp = now();
    let digest = sha256_hex(&input.content);
    let row = if let Some(existing) = message::Entity::find()
        .filter(message::Column::ThreadId.eq(run_row.thread_id))
        .filter(message::Column::ClientMessageId.eq(&client_message_id))
        .lock_exclusive()
        .one(&tx)
        .await?
    {
        if existing.run_id != Some(run_id)
            || existing.role != "assistant"
            || existing.status != "streaming"
            || !input.content.starts_with(&existing.content)
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent streaming message 不能回退或跨 run 更新: {client_message_id}"
            )));
        }
        let mut active: message::ActiveModel = existing.into();
        active.content_sha256 = Set(digest);
        active.content = Set(input.content);
        active.payload_json = Set(input.payload_json);
        active.updated_at = Set(timestamp);
        active.update(&tx).await?
    } else {
        message::ActiveModel {
            thread_id: Set(input.thread_id),
            turn_id: Set(input.turn_id),
            run_id: Set(input.run_id),
            role: Set(input.role),
            sender_kind: Set(input.sender_kind),
            sender_thread_id: Set(input.sender_thread_id),
            client_message_id: Set(Some(client_message_id)),
            content_sha256: Set(digest),
            content: Set(input.content),
            status: Set(input.status),
            payload_json: Set(input.payload_json),
            created_at: Set(timestamp),
            updated_at: Set(timestamp),
            ..Default::default()
        }
        .insert(&tx)
        .await?
    };
    append_message_event_on(&tx, &row, "message_delta", "streaming").await?;
    tx.commit().await?;
    Ok(row)
}

/// Finalizes a streaming assistant projection when the response also carries
/// tool calls. Returning an existing completed row makes retries idempotent.
pub async fn complete_assistant_streaming_message_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    client_message_id: &str,
    content: String,
    payload_json: Value,
) -> Result<Option<message::Model>, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let Some(row) = message::Entity::find()
        .filter(message::Column::ThreadId.eq(run_row.thread_id))
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::ClientMessageId.eq(client_message_id))
        .lock_exclusive()
        .one(&tx)
        .await?
    else {
        tx.commit().await?;
        return Ok(None);
    };
    if row.role != "assistant" || !matches!(row.status.as_str(), "streaming" | "complete") {
        return Err(AgentDbError::Conflict(format!(
            "Agent streaming message 状态不允许完成: {client_message_id}"
        )));
    }
    if row.status == "complete" {
        tx.commit().await?;
        return Ok(Some(row));
    }
    let timestamp = now();
    let mut active: message::ActiveModel = row.into();
    active.content_sha256 = Set(sha256_hex(&content));
    active.content = Set(content);
    active.status = Set("complete".into());
    active.payload_json = Set(Some(payload_json));
    active.updated_at = Set(timestamp);
    let updated = active.update(&tx).await?;
    append_message_event_on(&tx, &updated, "message_completed", "complete").await?;
    tx.commit().await?;
    Ok(Some(updated))
}

pub async fn interrupt_assistant_streaming_message_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    client_message_id: &str,
    reason: Option<&str>,
) -> Result<(), AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let Some(row) = message::Entity::find()
        .filter(message::Column::ThreadId.eq(run_row.thread_id))
        .filter(message::Column::RunId.eq(run_id))
        .filter(message::Column::ClientMessageId.eq(client_message_id))
        .filter(message::Column::Status.eq("streaming"))
        .lock_exclusive()
        .one(&tx)
        .await?
    else {
        tx.commit().await?;
        return Ok(());
    };
    let mut payload = row.payload_json.clone().unwrap_or_else(|| json!({}));
    if let Value::Object(object) = &mut payload {
        object.insert(
            "interrupted_reason".into(),
            reason.map_or(Value::Null, |value| Value::String(value.to_string())),
        );
    }
    let mut active: message::ActiveModel = row.into();
    active.status = Set("interrupted".into());
    active.payload_json = Set(Some(payload));
    active.updated_at = Set(now());
    let updated = active.update(&tx).await?;
    append_message_event_on(&tx, &updated, "message_interrupted", "interrupted").await?;
    tx.commit().await?;
    Ok(())
}

fn message_projection(row: &message::Model) -> Value {
    json!({
        "id": row.id.to_string(),
        "thread_id": row.thread_id.to_string(),
        "turn_id": row.turn_id.map(|id| id.to_string()),
        "run_id": row.run_id.map(|id| id.to_string()),
        "role": row.role,
        "status": row.status,
        "content": row.content,
        "source_kind": row.sender_kind,
        "source_id": row.sender_thread_id.map(|id| id.to_string()),
        "metadata": row.payload_json,
        "created_at": row.created_at.to_string(),
        "updated_at": row.updated_at.to_string(),
    })
}

async fn append_message_event_on<C: ConnectionTrait>(
    db: &C,
    row: &message::Model,
    kind: &str,
    status: &str,
) -> Result<event::Model, AgentDbError> {
    append_event_on(
        db,
        AgentEventInput {
            thread_id: row.thread_id,
            turn_id: row.turn_id,
            run_id: row.run_id,
            execution_id: None,
            delegation_id: None,
            message_id: Some(row.id),
            actor_kind: "agent".into(),
            actor_id: row.run_id.map(|id| id.to_string()),
            kind: kind.into(),
            status: status.into(),
            public_text: String::new(),
            payload_json: Some(json!({
                "message_id": row.id.to_string(),
                "message": message_projection(row),
            })),
        },
    )
    .await
}

async fn interrupt_streaming_messages_on<C: ConnectionTrait>(
    db: &C,
    run_ids: impl IntoIterator<Item = i64>,
    reason: Option<&str>,
) -> Result<(), AgentDbError> {
    let run_ids = run_ids.into_iter().collect::<Vec<_>>();
    if run_ids.is_empty() {
        return Ok(());
    }
    let rows = message::Entity::find()
        .filter(message::Column::RunId.is_in(run_ids))
        .filter(message::Column::Role.eq("assistant"))
        .filter(message::Column::Status.eq("streaming"))
        .lock_exclusive()
        .all(db)
        .await?;
    let timestamp = now();
    for row in rows {
        let mut payload = row.payload_json.clone().unwrap_or_else(|| json!({}));
        if let Value::Object(object) = &mut payload {
            object.insert(
                "interrupted_reason".into(),
                reason.map_or(Value::Null, |value| Value::String(value.to_string())),
            );
        }
        let mut active: message::ActiveModel = row.into();
        active.status = Set("interrupted".into());
        active.payload_json = Set(Some(payload));
        active.updated_at = Set(timestamp);
        let updated = active.update(db).await?;
        append_message_event_on(db, &updated, "message_interrupted", "interrupted").await?;
    }
    Ok(())
}

/// Atomically appends the input message and creates the first execution run.
/// Replaying the same client id returns the original turn/run without adding
/// another active writer to the thread.
pub async fn create_turn_with_run(
    db: &DatabaseConnection,
    input: AgentTurnInput,
) -> Result<(turn::Model, run::Model, message::Model), AgentDbError> {
    let tx = db.begin().await?;
    let rows = create_turn_with_run_on(&tx, input).await?;
    tx.commit().await?;
    Ok(rows)
}

async fn create_turn_with_run_on<C>(
    db: &C,
    input: AgentTurnInput,
) -> Result<(turn::Model, run::Model, message::Model), AgentDbError>
where
    C: ConnectionTrait,
{
    if input.client_message_id.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "client_message_id 不能为空".into(),
        ));
    }
    if input.message.content.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument("Agent 消息不能为空".into()));
    }
    let thread_row = thread::Entity::find_by_id(input.thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", input.thread_id))
        })?;

    if let Some(existing_turn) = turn::Entity::find()
        .filter(turn::Column::ThreadId.eq(input.thread_id))
        .filter(turn::Column::ClientMessageId.eq(&input.client_message_id))
        .one(db)
        .await?
    {
        let existing_run = run::Entity::find()
            .filter(run::Column::TurnId.eq(existing_turn.id))
            .order_by_desc(run::Column::Attempt)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("已有 Agent turn 缺少 run".into()))?;
        let existing_message = message::Entity::find_by_id(existing_turn.input_message_id)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("已有 Agent turn 缺少输入消息".into()))?;
        if existing_message.content_sha256 != sha256_hex(&input.message.content)
            || existing_message.role != input.message.role
        {
            return Err(AgentDbError::Conflict(format!(
                "client_message_id 已用于不同消息: {}",
                input.client_message_id
            )));
        }
        return Ok((existing_turn, existing_run, existing_message));
    }
    let active_rows = run::Entity::find()
        .filter(run::Column::ThreadId.eq(input.thread_id))
        .filter(run::Column::IsActive.eq(true))
        .lock_exclusive()
        .all(db)
        .await?;
    if active_rows.iter().any(|row| is_run_active(&row.status)) {
        return Err(AgentDbError::Conflict(
            "Agent thread 已有运行中的 turn".into(),
        ));
    }
    for row in active_rows {
        let mut inactive: run::ActiveModel = row.into();
        inactive.is_active = Set(false);
        inactive.worker_id = Set(None);
        inactive.lease_token = Set(None);
        inactive.lease_expires_at = Set(None);
        inactive.update(db).await?;
    }

    let mut message_input = input.message;
    message_input.thread_id = input.thread_id;
    message_input.client_message_id = Some(input.client_message_id.clone());
    let input_message = append_message_on(db, message_input).await?;
    let timestamp = now();
    let sequence = thread_row
        .next_turn_sequence
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent turn sequence 溢出".into()))?;
    let turn_row = turn::ActiveModel {
        thread_id: Set(input.thread_id),
        sequence: Set(sequence),
        client_message_id: Set(input.client_message_id),
        input_message_id: Set(input_message.id),
        final_message_id: Set(None),
        trigger: Set(input.trigger),
        goal: Set(input.goal),
        status: Set("queued".to_string()),
        execution_mode: Set(input.execution_mode),
        model_override_json: Set(input.model_override_json),
        error_message: Set(None),
        result_refs_json: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let run_row = run::ActiveModel {
        thread_id: Set(input.thread_id),
        turn_id: Set(turn_row.id),
        root_run_id: Set(None),
        parent_run_id: Set(None),
        attempt: Set(1),
        status: Set("queued".to_string()),
        waiting_reason: Set(None),
        is_active: Set(true),
        next_execution_sequence: Set(0),
        budget_json: Set(input.budget_json),
        usage_json: Set(json!({
            "runtime_snapshot_required": input.runtime_snapshot_required,
        })),
        worker_id: Set(None),
        lease_token: Set(None),
        lease_expires_at: Set(None),
        next_attempt_at: Set(timestamp),
        last_error: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        started_at: Set(None),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let mut input_message_active: message::ActiveModel = input_message.into();
    input_message_active.turn_id = Set(Some(turn_row.id));
    input_message_active.run_id = Set(Some(run_row.id));
    input_message_active.updated_at = Set(timestamp);
    let input_message = input_message_active.update(db).await?;
    let mut thread_active: thread::ActiveModel = thread_row.into();
    thread_active.status = Set("running".to_string());
    let revision = thread_active
        .revision
        .take()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 未设置".into()))?;
    thread_active.revision = Set(revision
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 溢出".into()))?);
    thread_active.next_turn_sequence = Set(sequence);
    thread_active.updated_at = Set(timestamp);
    thread_active.update(db).await?;
    Ok((turn_row, run_row, input_message))
}

pub async fn append_event(
    db: &DatabaseConnection,
    input: AgentEventInput,
) -> Result<event::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = append_event_on(&tx, input).await?;
    tx.commit().await?;
    Ok(row)
}

async fn append_event_on<C>(db: &C, input: AgentEventInput) -> Result<event::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let thread_row = thread::Entity::find_by_id(input.thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", input.thread_id))
        })?;
    let sequence = thread_row
        .next_event_sequence
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent event sequence 溢出".into()))?;
    let timestamp = now();
    let row = event::ActiveModel {
        thread_id: Set(input.thread_id),
        sequence: Set(sequence),
        turn_id: Set(input.turn_id),
        run_id: Set(input.run_id),
        execution_id: Set(input.execution_id),
        delegation_id: Set(input.delegation_id),
        message_id: Set(input.message_id),
        actor_kind: Set(input.actor_kind),
        actor_id: Set(input.actor_id),
        kind: Set(input.kind),
        status: Set(input.status),
        public_text: Set(input.public_text),
        payload_json: Set(input.payload_json),
        created_at: Set(timestamp),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let mut active: thread::ActiveModel = thread_row.into();
    active.next_event_sequence = Set(sequence);
    active.updated_at = Set(timestamp);
    active.update(db).await?;
    Ok(row)
}

async fn append_run_event_on<C>(
    db: &C,
    run_row: &run::Model,
    kind: &str,
    status: &str,
    public_text: &str,
    execution_id: Option<i64>,
    delegation_id: Option<i64>,
    message_id: Option<i64>,
    payload_json: Option<Value>,
) -> Result<event::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    append_event_on(
        db,
        AgentEventInput {
            thread_id: run_row.thread_id,
            turn_id: Some(run_row.turn_id),
            run_id: Some(run_row.id),
            execution_id,
            delegation_id,
            message_id,
            actor_kind: "agent".into(),
            actor_id: Some(run_row.id.to_string()),
            kind: kind.into(),
            status: status.into(),
            public_text: public_text.into(),
            payload_json,
        },
    )
    .await
}

/// Appends a public worker event only while the caller still owns the Run.
///
/// The Thread is locked before the Run, matching turn admission and all
/// terminal transitions that also refresh the Thread projection.
pub async fn append_event_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    input: AgentEventInput,
) -> Result<event::Model, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if input.thread_id != identity.thread_id || input.run_id != Some(run_id) {
        return Err(AgentDbError::Conflict(format!(
            "Agent event 与 run 不一致: {run_id}"
        )));
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let row = append_event_on(&tx, input).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn events_after<C>(
    db: &C,
    thread_id: i64,
    after_sequence: i64,
    limit: u64,
) -> Result<Vec<event::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    event::Entity::find()
        .filter(event::Column::ThreadId.eq(thread_id))
        .filter(event::Column::Sequence.gt(after_sequence))
        .order_by_asc(event::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn latest_events_by_thread<C>(
    db: &C,
    thread_id: i64,
    limit: u64,
) -> Result<Vec<event::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut rows = event::Entity::find()
        .filter(event::Column::ThreadId.eq(thread_id))
        .order_by_desc(event::Column::Sequence)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)?;
    rows.reverse();
    Ok(rows)
}

pub async fn enqueue_inbox<C>(db: &C, input: AgentInboxInput) -> Result<inbox::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if input.dedupe_key.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "inbox dedupe_key 不能为空".into(),
        ));
    }
    let timestamp = now();
    let digest = sha256_hex(&input.content);
    let row = inbox::ActiveModel {
        thread_id: Set(input.thread_id),
        turn_id: Set(input.turn_id),
        run_id: Set(input.run_id),
        source_kind: Set(input.source_kind),
        source_thread_id: Set(input.source_thread_id),
        inbox_kind: Set(input.inbox_kind),
        delivery_mode: Set(input.delivery_mode),
        priority: Set(input.priority),
        dedupe_key: Set(input.dedupe_key.clone()),
        content_sha256: Set(digest.clone()),
        content: Set(input.content),
        payload_json: Set(input.payload_json),
        status: Set("pending".to_string()),
        available_at: Set(input.available_at.unwrap_or(timestamp)),
        claim_token: Set(None),
        claim_expires_at: Set(None),
        created_at: Set(timestamp),
        claimed_at: Set(None),
        consumed_at: Set(None),
        ..Default::default()
    };
    inbox::Entity::insert(row)
        .on_conflict(
            OnConflict::columns([inbox::Column::ThreadId, inbox::Column::DedupeKey])
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec_without_returning(db)
        .await?;
    let existing = inbox::Entity::find()
        .filter(inbox::Column::ThreadId.eq(input.thread_id))
        .filter(inbox::Column::DedupeKey.eq(&input.dedupe_key))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "inbox 插入或幂等冲突后无法重读: thread={}, dedupe_key={}",
                input.thread_id, input.dedupe_key
            ))
        })?;
    if existing.content_sha256 != digest {
        return Err(AgentDbError::Conflict(format!(
            "inbox dedupe_key 已用于不同内容: {}",
            input.dedupe_key
        )));
    }
    Ok(existing)
}

pub async fn claim_inbox(
    db: &DatabaseConnection,
    thread_id: Option<i64>,
) -> Result<Option<inbox::Model>, AgentDbError> {
    let tx = db.begin().await?;
    let timestamp = now();
    let claimable = sea_orm::Condition::any()
        .add(
            sea_orm::Condition::all()
                .add(inbox::Column::Status.eq("pending"))
                .add(inbox::Column::AvailableAt.lte(timestamp)),
        )
        .add(
            sea_orm::Condition::all()
                .add(inbox::Column::Status.eq("claimed"))
                .add(inbox::Column::ClaimExpiresAt.lte(timestamp)),
        );
    let mut query = inbox::Entity::find()
        .filter(claimable)
        .order_by_desc(inbox::Column::Priority)
        .order_by_asc(inbox::Column::Id)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked);
    if let Some(thread_id) = thread_id {
        query = query.filter(inbox::Column::ThreadId.eq(thread_id));
    }
    let Some(row) = query.one(&tx).await? else {
        tx.commit().await?;
        return Ok(None);
    };
    let token = uuid::Uuid::new_v4().to_string();
    let mut active: inbox::ActiveModel = row.into();
    active.status = Set("claimed".to_string());
    active.claim_token = Set(Some(token));
    active.claim_expires_at = Set(Some(timestamp + Duration::seconds(INBOX_LEASE_SECONDS)));
    active.claimed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(Some(row))
}

fn validate_inbox_claim(
    row: &inbox::Model,
    claim_token: &str,
    timestamp: NaiveDateTime,
) -> Result<(), AgentDbError> {
    if claim_token.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "inbox claim_token 不能为空".into(),
        ));
    }
    if row.status != "claimed"
        || row.claim_token.as_deref() != Some(claim_token)
        || row
            .claim_expires_at
            .is_none_or(|expires_at| expires_at <= timestamp)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent inbox claim 已失效: {}",
            row.id
        )));
    }
    Ok(())
}

pub async fn consume_inbox(
    db: &DatabaseConnection,
    inbox_id: i64,
    claim_token: &str,
) -> Result<inbox::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = inbox::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let timestamp = now();
    validate_inbox_claim(&row, claim_token, timestamp)?;
    let mut active: inbox::ActiveModel = row.into();
    active.status = Set("consumed".to_string());
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.consumed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

/// Atomically turns a claimed inbox item into a durable Agent message and
/// consumes the item.  The message is written in the same transaction as the
/// claim transition, so a worker crash cannot lose an observation between
/// consuming the inbox and rebuilding model history.
pub async fn consume_inbox_with_message(
    db: &DatabaseConnection,
    inbox_id: i64,
    claim_token: &str,
    run_lease_token: &str,
    message: AgentMessageInput,
) -> Result<(inbox::Model, message::Model), AgentDbError> {
    let identity = inbox::Entity::find_by_id(inbox_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let tx = db.begin().await?;

    // Keep the shared lock order (Thread → Run → Inbox).  Cancellation and
    // child completion use the same order before touching these records.
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    if let Some(run_id) = identity.run_id {
        let run_row = run::Entity::find_by_id(run_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
        // Consuming an inbox item mutates the durable conversation history.
        // It therefore has to be fenced by the same live Run lease as every
        // other worker write; an expired worker must roll back both message
        // insertion and inbox consumption.
        validate_live_run_lease(&run_row, run_lease_token, now())?;
    }
    let row = inbox::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    validate_inbox_claim(&row, claim_token, now())?;
    if message.thread_id != row.thread_id
        || message.turn_id != row.turn_id
        || message.run_id != row.run_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent inbox durable message 与 inbox 不一致: {inbox_id}"
        )));
    }
    let durable_message = append_message_on(&tx, message).await?;
    let timestamp = now();
    let mut active: inbox::ActiveModel = row.into();
    active.status = Set("consumed".to_string());
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.consumed_at = Set(Some(timestamp));
    let consumed = active.update(&tx).await?;
    tx.commit().await?;
    Ok((consumed, durable_message))
}

pub async fn supersede_inbox(
    db: &DatabaseConnection,
    inbox_id: i64,
    claim_token: Option<&str>,
) -> Result<inbox::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = inbox::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let timestamp = now();
    match row.status.as_str() {
        "pending" => {}
        "claimed" => validate_inbox_claim(
            &row,
            claim_token.ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent inbox claim token 缺失: {inbox_id}"))
            })?,
            timestamp,
        )?,
        "superseded" => {
            tx.commit().await?;
            return Ok(row);
        }
        status => {
            return Err(AgentDbError::Conflict(format!(
                "Agent inbox 无法从 {status} 变为 superseded: {inbox_id}"
            )));
        }
    }
    let mut active: inbox::ActiveModel = row.into();
    active.status = Set("superseded".to_string());
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.consumed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn release_inbox_claim(
    db: &DatabaseConnection,
    inbox_id: i64,
    claim_token: &str,
    available_at: Option<NaiveDateTime>,
) -> Result<inbox::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = inbox::Entity::find_by_id(inbox_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent inbox 不存在: {inbox_id}")))?;
    let timestamp = now();
    validate_inbox_claim(&row, claim_token, timestamp)?;
    let mut active: inbox::ActiveModel = row.into();
    active.status = Set("pending".to_string());
    active.available_at = Set(available_at.unwrap_or(timestamp));
    active.claim_token = Set(None);
    active.claim_expires_at = Set(None);
    active.claimed_at = Set(None);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn claim_runnable_run(
    db: &DatabaseConnection,
    worker_id: &str,
) -> Result<Option<run::Model>, AgentDbError> {
    if worker_id.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument("worker_id 不能为空".into()));
    }
    let tx = db.begin().await?;
    let timestamp = now();
    let claimable = sea_orm::Condition::all()
        .add(run::Column::Status.eq("queued"))
        .add(run::Column::NextAttemptAt.lte(timestamp))
        .add(
            sea_orm::Condition::any()
                .add(run::Column::LeaseExpiresAt.is_null())
                .add(run::Column::LeaseExpiresAt.lte(timestamp)),
        );
    let row = run::Entity::find()
        .filter(run::Column::IsActive.eq(true))
        .filter(claimable)
        .order_by_asc(run::Column::NextAttemptAt)
        .order_by_asc(run::Column::Id)
        .lock_with_behavior(LockType::Update, LockBehavior::SkipLocked)
        .one(&tx)
        .await?;
    let Some(row) = row else {
        tx.commit().await?;
        return Ok(None);
    };
    let token = uuid::Uuid::new_v4().to_string();
    let started_at = row.started_at.or(Some(timestamp));
    let mut active: run::ActiveModel = row.into();
    active.worker_id = Set(Some(worker_id.to_string()));
    active.lease_token = Set(Some(token));
    active.lease_expires_at = Set(Some(timestamp + Duration::seconds(RUN_LEASE_SECONDS)));
    active.status = Set("running".to_string());
    active.started_at = Set(started_at);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(Some(row))
}

pub async fn renew_run_lease<C>(db: &C, run_id: i64, lease_token: &str) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let timestamp = now();
    let result = run::Entity::update_many()
        .col_expr(
            run::Column::LeaseExpiresAt,
            sea_orm::sea_query::Expr::value(timestamp + Duration::seconds(RUN_LEASE_SECONDS)),
        )
        .col_expr(
            run::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(timestamp),
        )
        .filter(run::Column::Id.eq(run_id))
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("running"))
        .filter(run::Column::LeaseToken.eq(lease_token))
        .filter(run::Column::LeaseExpiresAt.gt(timestamp))
        .exec(db)
        .await?;
    if result.rows_affected != 1 {
        return Err(AgentDbError::Conflict(format!(
            "Agent run lease 已失效: {run_id}"
        )));
    }
    Ok(())
}

/// The original editor may resolve only this waiting execution. It temporarily
/// owns the same Run lease; the normal worker cannot dispatch it concurrently.
pub async fn claim_editor_execution(
    db: &DatabaseConnection,
    user_id: i64,
    run_id: i64,
    execution_id: i64,
) -> Result<run::Model, AgentDbError> {
    resolve_editor_wait(db, user_id, run_id, execution_id, false).await
}

/// Reconcile durable editor waits after disconnect or restart. Expiry and a
/// live response contend on the same locks; only one can resolve the request.
pub async fn expire_editor_requests(
    db: &DatabaseConnection,
    limit: u64,
) -> Result<usize, AgentDbError> {
    let rows = run::Entity::find()
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("waiting"))
        .filter(run::Column::WaitingReason.eq("external"))
        .filter(run::Column::UpdatedAt.lte(now() - Duration::seconds(EDITOR_RESPONSE_TTL_SECONDS)))
        .filter(sea_orm::sea_query::Expr::cust("jsonb_typeof((SELECT payload_json->'editor_request' FROM agent_event WHERE run_id = agent_run.id AND kind = 'run_waiting' ORDER BY sequence DESC LIMIT 1)) = 'object'"))
        .order_by_asc(run::Column::UpdatedAt).limit(limit).all(db).await?;
    let mut expired = 0;
    for row in rows {
        let request = latest_editor_wait(db, row.id).await?;
        let execution_id = request
            .payload_json
            .as_ref()
            .and_then(|value| value.pointer("/editor_request/execution_id"))
            .and_then(Value::as_str)
            .and_then(|id| id.parse::<i64>().ok())
            .ok_or_else(|| AgentDbError::InvalidArgument("编辑器等待缺少 execution_id".into()))?;
        let owner = thread::Entity::find_by_id(row.thread_id)
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("Agent Thread 不存在".into()))?;
        match resolve_editor_wait(db, owner.owner_user_id, row.id, execution_id, true).await {
            Ok(_) => expired += 1,
            Err(AgentDbError::Conflict(_)) => {} // A response or cancellation won.
            Err(error) => return Err(error),
        }
    }
    Ok(expired)
}

async fn latest_editor_wait<C: ConnectionTrait>(
    db: &C,
    run_id: i64,
) -> Result<event::Model, AgentDbError> {
    event::Entity::find()
        .filter(event::Column::RunId.eq(run_id))
        .filter(event::Column::Kind.eq("run_waiting"))
        .order_by_desc(event::Column::Sequence)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::Conflict("Run 没有待响应的编辑器请求".into()))
}

async fn resolve_editor_wait(
    db: &DatabaseConnection,
    user_id: i64,
    run_id: i64,
    execution_id: i64,
    expire: bool,
) -> Result<run::Model, AgentDbError> {
    let identity = run_by_id(db, run_id)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    let tx = db.begin().await?;
    let thread = thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Thread 不存在".into()))?;
    if thread.owner_user_id != user_id {
        return Err(AgentDbError::Conflict(
            "编辑器不能响应其他用户的执行".into(),
        ));
    }
    let run = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Run 不存在".into()))?;
    if !run.is_active
        || run.status != "waiting"
        || run.waiting_reason.as_deref() != Some("external")
    {
        return Err(AgentDbError::Conflict("编辑器请求已失效或正在处理".into()));
    }
    let wait = latest_editor_wait(&tx, run_id).await?;
    let timestamp = now();
    if expire != (timestamp >= wait.created_at + Duration::seconds(EDITOR_RESPONSE_TTL_SECONDS)) {
        return Err(AgentDbError::Conflict(
            if expire {
                "编辑器请求尚未过期"
            } else {
                "原编辑器响应已过期"
            }
            .into(),
        ));
    }
    let request = wait
        .payload_json
        .and_then(|payload| payload.get("editor_request").cloned());
    if request
        .as_ref()
        .and_then(|request| request.get("execution_id"))
        .and_then(Value::as_str)
        != Some(execution_id.to_string().as_str())
    {
        return Err(AgentDbError::Conflict("不是当前等待的编辑器执行".into()));
    }
    let execution = execution::Entity::find_by_id(execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent Execution 不存在".into()))?;
    if execution.run_id != run_id
        || !matches!(
            execution.status.as_str(),
            "prepared" | "awaiting_user" | "dispatching"
        )
    {
        return Err(AgentDbError::Conflict(
            "编辑器执行已结束或不属于当前任务".into(),
        ));
    }
    if expire {
        let error_message = "原编辑器离线或未及时响应，请回到发起请求的视图后重试；未执行写入";
        let mut active: execution::ActiveModel = execution.into();
        active.status = Set("failed".into());
        active.error_message = Set(Some(error_message.into()));
        active.updated_at = Set(timestamp);
        active.completed_at = Set(Some(timestamp));
        active.update(&tx).await?;
        append_run_event_on(
            &tx,
            &run,
            "editor_request_expired",
            "failed",
            "原编辑器未及时响应，工具未执行",
            Some(execution_id),
            None,
            None,
            None,
        )
        .await?;
        let failed = fail_run_on(&tx, run, Some(error_message.to_string()), timestamp).await?;
        tx.commit().await?;
        return Ok(failed);
    }
    let mut active: run::ActiveModel = run.into();
    active.status = Set("running".into());
    active.waiting_reason = Set(None);
    active.worker_id = Set(Some("editor-response".into()));
    active.lease_token = Set(Some(uuid::Uuid::new_v4().to_string()));
    active.lease_expires_at = Set(Some(timestamp + Duration::seconds(RUN_LEASE_SECONDS)));
    active.updated_at = Set(timestamp);
    let run = active.update(&tx).await?;
    tx.commit().await?;
    Ok(run)
}

async fn refresh_thread_status_on<C>(db: &C, thread_id: i64) -> Result<thread::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let thread_row = thread::Entity::find_by_id(thread_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    if thread_row.archived_at.is_some() || thread_row.status == "archived" {
        return Ok(thread_row);
    }
    let active_runs = active_runs_by_thread(db, thread_id).await?;
    if active_runs.len() > 1 {
        return Err(AgentDbError::Conflict(format!(
            "Agent thread 存在多个 active run: {thread_id}"
        )));
    }
    let status = projected_thread_status(active_runs.first())?;
    if thread_row.status == status {
        return Ok(thread_row);
    }
    let timestamp = now();
    let mut active: thread::ActiveModel = thread_row.into();
    active.status = Set(status.to_string());
    active.updated_at = Set(timestamp);
    let revision = active
        .revision
        .take()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 未设置".into()))?;
    active.revision = Set(revision
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent thread revision 溢出".into()))?);
    let updated = active.update(db).await.map_err(AgentDbError::Database)?;
    Ok(updated)
}

async fn lock_threads_on<C>(
    db: &C,
    thread_ids: impl IntoIterator<Item = i64>,
) -> Result<(), AgentDbError>
where
    C: ConnectionTrait,
{
    let mut thread_ids = thread_ids.into_iter().collect::<Vec<_>>();
    thread_ids.sort_unstable();
    thread_ids.dedup();
    for thread_id in thread_ids {
        thread::Entity::find_by_id(thread_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(|| AgentDbError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
    }
    Ok(())
}

async fn lock_runs_on<C>(
    db: &C,
    run_ids: impl IntoIterator<Item = i64>,
) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut run_ids = run_ids.into_iter().collect::<Vec<_>>();
    run_ids.sort_unstable();
    run_ids.dedup();
    let mut rows = Vec::with_capacity(run_ids.len());
    for run_id in run_ids {
        rows.push(
            run::Entity::find_by_id(run_id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?,
        );
    }
    Ok(rows)
}

async fn lock_turns_on<C>(
    db: &C,
    turn_ids: impl IntoIterator<Item = i64>,
) -> Result<Vec<turn::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut turn_ids = turn_ids.into_iter().collect::<Vec<_>>();
    turn_ids.sort_unstable();
    turn_ids.dedup();
    let mut rows = Vec::with_capacity(turn_ids.len());
    for turn_id in turn_ids {
        rows.push(
            turn::Entity::find_by_id(turn_id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {turn_id}")))?,
        );
    }
    Ok(rows)
}

async fn lock_delegations_on<C>(
    db: &C,
    delegation_ids: impl IntoIterator<Item = i64>,
) -> Result<Vec<delegation::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut delegation_ids = delegation_ids.into_iter().collect::<Vec<_>>();
    delegation_ids.sort_unstable();
    delegation_ids.dedup();
    let mut rows = Vec::with_capacity(delegation_ids.len());
    for delegation_id in delegation_ids {
        rows.push(
            delegation::Entity::find_by_id(delegation_id)
                .lock_exclusive()
                .one(db)
                .await?
                .ok_or_else(|| {
                    AgentDbError::NotFound(format!("Agent delegation 不存在: {delegation_id}"))
                })?,
        );
    }
    Ok(rows)
}

async fn update_run_state_on<C>(
    db: &C,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
) -> Result<run::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_run_status(status)?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if is_run_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 已终止，不能从 {} 变为 {status}: {run_id}",
            row.status
        )));
    }
    if let Some(token) = lease_token {
        validate_live_run_lease(&row, token, now())?;
    }
    let terminal = is_run_terminal(status);
    let timestamp = now();
    let mut active: run::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.waiting_reason = Set(waiting_reason.map(str::to_string));
    active.last_error = Set(error_message.map(str::to_string));
    active.is_active = Set(!terminal);
    active.updated_at = Set(timestamp);
    if terminal {
        active.completed_at = Set(Some(timestamp));
        active.worker_id = Set(None);
        active.lease_token = Set(None);
        active.lease_expires_at = Set(None);
    } else {
        active.completed_at = Set(None);
        if matches!(status, "queued" | "waiting" | "pausing" | "paused") {
            active.worker_id = Set(None);
            active.lease_token = Set(None);
            active.lease_expires_at = Set(None);
        }
        if status == "queued" {
            active.next_attempt_at = Set(timestamp);
        }
    }
    let updated = active.update(db).await.map_err(AgentDbError::Database)?;
    let lifecycle = match status {
        "queued" => Some(("run_queued", "queued", "Agent run 已排队")),
        "waiting" => Some(("run_waiting", "waiting", "Agent run 正在等待")),
        "completed" => Some(("run_completed", "completed", "Agent run 已完成")),
        "failed" => Some(("run_failed", "failed", "Agent run 执行失败")),
        "cancelled" => Some(("run_cancelled", "cancelled", "Agent run 已取消")),
        "recovery_required" => Some((
            "run_recovery_required",
            "recovery_required",
            "Agent run 需要显式恢复或对账",
        )),
        _ => None,
    };
    if let Some((kind, event_status, public_text)) = lifecycle {
        append_run_event_on(
            db,
            &updated,
            kind,
            event_status,
            public_text,
            None,
            None,
            None,
            None,
        )
        .await?;
    }
    Ok(updated)
}

pub async fn update_run_state(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: Option<&str>,
    status: &str,
    waiting_reason: Option<&str>,
    error_message: Option<&str>,
) -> Result<run::Model, AgentDbError> {
    validate_run_status(status)?;
    let current = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(current.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", current.thread_id))
        })?;
    let run_row = update_run_state_on(
        &tx,
        run_id,
        lease_token,
        status,
        waiting_reason,
        error_message,
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    tx.commit().await?;
    Ok(run_row)
}

/// Moves a leased worker run to `waiting` and releases the lease in one
/// transaction.  Waiting runs must not retain a worker lease: approvals and
/// child results should be able to wake them immediately.
pub async fn wait_run_for_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    waiting_reason: &str,
) -> Result<run::Model, AgentDbError> {
    wait_run_for_lease_with_request(db, run_id, lease_token, waiting_reason, None).await
}

/// Publishes the external request in the same transaction that releases the
/// Run, so a fast UI response cannot arrive before the wait is durable.
pub async fn wait_run_for_lease_with_request(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    waiting_reason: &str,
    editor_request: Option<Value>,
) -> Result<run::Model, AgentDbError> {
    if waiting_reason.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent waiting_reason 不能为空".into(),
        ));
    }
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let current = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&current, lease_token, timestamp)?;
    let mut active: run::ActiveModel = current.into();
    active.status = Set("waiting".to_string());
    active.waiting_reason = Set(Some(waiting_reason.to_string()));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_waiting",
        "waiting",
        "Agent run 正在等待",
        None,
        None,
        None,
        Some(json!({ "waiting_reason": waiting_reason, "editor_request": editor_request })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
    tx.commit().await?;
    Ok(row)
}

/// Requeues an active waiting run and releases any stale lease.  This is used
/// by approval/child-result delivery and intentionally has no worker token:
/// the durable inbox is the authorization boundary for the wake-up.
/// Requeues a waiting Run only when its current wait reason matches the
/// supplied reason. User/external input may release a Run waiting for input,
/// but must not bypass an approval, child result, budget, or reconciliation
/// gate. A concurrent resolver that already changed the state is an ordinary
/// no-op rather than a failed message delivery.
pub async fn wake_waiting_run_for_reason(
    db: &DatabaseConnection,
    run_id: i64,
    waiting_reason: &str,
) -> Result<Option<run::Model>, AgentDbError> {
    if waiting_reason.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent waiting_reason 不能为空".into(),
        ));
    }
    let tx = db.begin().await?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if !row.is_active
        || row.status != "waiting"
        || row.waiting_reason.as_deref() != Some(waiting_reason)
    {
        tx.commit().await?;
        return Ok(None);
    }
    let timestamp = now();
    let mut active: run::ActiveModel = row.into();
    active.status = Set("queued".to_string());
    active.waiting_reason = Set(None);
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.next_attempt_at = Set(timestamp);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_woken",
        "queued",
        "Agent run 已被外部输入唤醒",
        None,
        None,
        None,
        Some(json!({ "waiting_reason": waiting_reason })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
    tx.commit().await?;
    Ok(Some(row))
}

/// Fenced failure transition for a worker.  Keeping Turn and Run terminal
/// updates in one transaction avoids an observable half-failed turn.
async fn fail_run_on<C: ConnectionTrait>(
    db: &C,
    row: run::Model,
    error_message: Option<String>,
    timestamp: NaiveDateTime,
) -> Result<run::Model, AgentDbError> {
    let turn_row = turn::Entity::find_by_id(row.turn_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", row.turn_id)))?;
    let mut turn_active: turn::ActiveModel = turn_row.into();
    turn_active.status = Set("failed".to_string());
    turn_active.error_message = Set(error_message.clone());
    turn_active.updated_at = Set(timestamp);
    turn_active.completed_at = Set(Some(timestamp));
    turn_active.update(db).await?;

    let mut active: run::ActiveModel = row.into();
    active.status = Set("failed".to_string());
    active.waiting_reason = Set(None);
    active.is_active = Set(false);
    active.last_error = Set(error_message);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    let row = active.update(db).await?;
    terminate_executions_on(
        db,
        [row.id],
        ExecutionTermination::Failed,
        row.last_error.as_deref(),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(db, [row.id], row.last_error.as_deref()).await?;
    append_run_event_on(
        db,
        &row,
        "run_failed",
        "failed",
        "Agent run 执行失败",
        None,
        None,
        None,
        row.last_error
            .clone()
            .map(|error| json!({ "error": error })),
    )
    .await?;
    let _ = refresh_thread_status_on(db, row.thread_id).await?;
    Ok(row)
}

pub async fn fail_run_for_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    error_message: Option<String>,
) -> Result<run::Model, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    let row = fail_run_on(&tx, row, error_message, timestamp).await?;
    tx.commit().await?;
    Ok(row)
}

/// Fenced transition used when a worker cannot safely resume (for example the
/// process-local runtime snapshot is gone).  The state is terminal and must be
/// reconciled explicitly instead of silently retrying an unknown side effect.
pub async fn recovery_required_for_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    reason: Option<String>,
) -> Result<run::Model, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    let turn_row = turn::Entity::find_by_id(row.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", row.turn_id)))?;
    let mut turn_active: turn::ActiveModel = turn_row.into();
    turn_active.status = Set("failed".to_string());
    turn_active.error_message = Set(reason.clone());
    turn_active.updated_at = Set(timestamp);
    turn_active.completed_at = Set(Some(timestamp));
    turn_active.update(&tx).await?;
    let mut active: run::ActiveModel = row.into();
    active.status = Set("recovery_required".to_string());
    active.waiting_reason = Set(Some("reconcile".to_string()));
    active.is_active = Set(false);
    active.last_error = Set(reason);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    let row = active.update(&tx).await?;
    terminate_executions_on(
        &tx,
        [row.id],
        ExecutionTermination::RecoveryRequired,
        row.last_error.as_deref(),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(&tx, [row.id], row.last_error.as_deref()).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_recovery_required",
        "recovery_required",
        "Agent run 需要显式恢复或对账",
        None,
        None,
        None,
        row.last_error
            .clone()
            .map(|error| json!({ "error": error })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
    tx.commit().await?;
    Ok(row)
}

pub async fn recover_expired_run(
    db: &DatabaseConnection,
    run_id: i64,
    error_message: Option<&str>,
) -> Result<run::Model, AgentDbError> {
    let current = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let parent_identity = if let Some(parent_run_id) = current.parent_run_id {
        let delegation = delegation::Entity::find()
            .filter(delegation::Column::ChildRunId.eq(current.id))
            .one(db)
            .await?
            .ok_or_else(|| {
                AgentDbError::Conflict(format!(
                    "Agent child run 缺少 delegation，无法恢复: {}",
                    current.id
                ))
            })?;
        if delegation.parent_run_id != parent_run_id {
            return Err(AgentDbError::Conflict(format!(
                "Agent child delegation parent 不一致: {}",
                current.id
            )));
        }
        Some(
            run::Entity::find_by_id(parent_run_id)
                .one(db)
                .await?
                .ok_or_else(|| {
                    AgentDbError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
                })?,
        )
    } else {
        None
    };
    let tx = db.begin().await?;
    let mut thread_ids = vec![current.thread_id];
    if let Some(parent) = parent_identity.as_ref() {
        thread_ids.push(parent.thread_id);
    }
    lock_threads_on(&tx, thread_ids.iter().copied()).await?;
    let run_ids = parent_identity
        .as_ref()
        .map(|parent| vec![current.id, parent.id])
        .unwrap_or_else(|| vec![current.id]);
    let locked_runs = lock_runs_on(&tx, run_ids).await?;
    let row = locked_runs
        .iter()
        .find(|row| row.id == run_id)
        .cloned()
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    if row.status == "recovery_required" && !row.is_active {
        let _ = refresh_thread_status_on(&tx, row.thread_id).await?;
        tx.commit().await?;
        return Ok(row);
    }
    let timestamp = now();
    if !row.is_active
        || row.status != "running"
        || row
            .lease_expires_at
            .is_none_or(|expires_at| expires_at > timestamp)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent run 不满足过期恢复条件: {run_id}"
        )));
    }
    let parent_run = parent_identity
        .and_then(|parent| locked_runs.iter().find(|row| row.id == parent.id).cloned());
    let mut active: run::ActiveModel = row.into();
    active.status = Set("recovery_required".to_string());
    active.waiting_reason = Set(Some("reconcile".to_string()));
    active.is_active = Set(false);
    active.last_error = Set(error_message.map(str::to_string));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    let row = active.update(&tx).await?;
    terminate_executions_on(
        &tx,
        [row.id],
        ExecutionTermination::RecoveryRequired,
        row.last_error.as_deref(),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(&tx, [row.id], row.last_error.as_deref()).await?;
    append_run_event_on(
        &tx,
        &row,
        "run_recovery_required",
        "recovery_required",
        "Agent run 需要显式恢复或对账",
        None,
        None,
        None,
        row.last_error
            .clone()
            .map(|error| json!({ "error": error })),
    )
    .await?;
    if let Some(parent_run) = parent_run {
        recover_child_delegation_on(&tx, &row, &parent_run, error_message, timestamp).await?;
    }
    for thread_id in thread_ids {
        let _ = refresh_thread_status_on(&tx, thread_id).await?;
    }
    tx.commit().await?;
    Ok(row)
}

/// Lists expired running leases for the recovery worker.
///
/// This is deliberately a read-only scan: `recover_expired_run` performs the
/// exclusive lock and revalidates expiry before changing durable state.
pub async fn expired_running_runs<C>(db: &C, limit: u64) -> Result<Vec<run::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    if limit == 0 {
        return Err(AgentDbError::InvalidArgument(
            "expired running run limit 必须大于 0".into(),
        ));
    }
    run::Entity::find()
        .filter(run::Column::IsActive.eq(true))
        .filter(run::Column::Status.eq("running"))
        .filter(run::Column::LeaseExpiresAt.lte(now()))
        .order_by_asc(run::Column::LeaseExpiresAt)
        .order_by_asc(run::Column::Id)
        .limit(limit)
        .all(db)
        .await
        .map_err(AgentDbError::Database)
}

pub async fn cancel_run_tree(
    db: &DatabaseConnection,
    root_run_id: i64,
    error_message: Option<&str>,
) -> Result<Vec<run::Model>, AgentDbError> {
    let root = run::Entity::find_by_id(root_run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {root_run_id}")))?;
    let tree_root_id = root.root_run_id.unwrap_or(root.id);
    let tree_rows = run::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(run::Column::Id.eq(tree_root_id))
                .add(run::Column::RootRunId.eq(tree_root_id)),
        )
        .all(db)
        .await?;
    let parent_by_run = tree_rows
        .iter()
        .map(|row| (row.id, row.parent_run_id))
        .collect::<std::collections::HashMap<_, _>>();
    let visited = tree_rows
        .iter()
        .filter(|row| {
            let mut current = Some(row.id);
            let mut seen = HashSet::new();
            while let Some(run_id) = current {
                if run_id == root_run_id {
                    return true;
                }
                if !seen.insert(run_id) {
                    return false;
                }
                current = parent_by_run.get(&run_id).copied().flatten();
            }
            false
        })
        .map(|row| row.id)
        .collect::<HashSet<_>>();
    if !visited.contains(&root_run_id) {
        return Err(AgentDbError::Conflict(format!(
            "Agent run lineage 已损坏: {root_run_id}"
        )));
    }
    let thread_ids = tree_rows
        .iter()
        .map(|row| row.thread_id)
        .collect::<HashSet<_>>();
    let delegation_rows = delegation::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(delegation::Column::ParentRunId.is_in(visited.iter().copied()))
                .add(delegation::Column::ChildRunId.is_in(visited.iter().copied())),
        )
        .all(db)
        .await?;
    let delegation_ids = delegation_rows.iter().map(|row| row.id).collect::<Vec<_>>();
    // When a child is cancelled directly, its parent is outside `visited`.
    // Lock those parent Threads/Runs in the same transaction so cancellation
    // can atomically deliver a tool observation and wake the waiting parent.
    let boundary_parent_run_ids = delegation_rows
        .iter()
        .filter_map(|row| {
            row.child_run_id
                .filter(|child_id| visited.contains(child_id))
                .and_then(|_| (!visited.contains(&row.parent_run_id)).then_some(row.parent_run_id))
        })
        .collect::<HashSet<_>>();
    let boundary_parent_rows = if boundary_parent_run_ids.is_empty() {
        Vec::new()
    } else {
        run::Entity::find()
            .filter(run::Column::Id.is_in(boundary_parent_run_ids.iter().copied()))
            .all(db)
            .await?
    };
    if boundary_parent_rows.len() != boundary_parent_run_ids.len() {
        return Err(AgentDbError::Conflict(
            "Agent child cancellation 缺少 parent run".into(),
        ));
    }
    let mut lock_thread_ids = thread_ids.clone();
    lock_thread_ids.extend(boundary_parent_rows.iter().map(|row| row.thread_id));

    let tx = db.begin().await?;
    lock_threads_on(&tx, lock_thread_ids.iter().copied()).await?;
    // Re-scan after every candidate Thread is locked. Child creation follows
    // the same Thread-before-Run order, so no new descendant can cross this
    // point. If the pre-scan missed one, abort the transaction and retry with
    // the expanded lock set instead of leaving a live child behind.
    let locked_tree_rows = run::Entity::find()
        .filter(
            sea_orm::Condition::any()
                .add(run::Column::Id.eq(tree_root_id))
                .add(run::Column::RootRunId.eq(tree_root_id)),
        )
        .all(&tx)
        .await?;
    let locked_parent_by_run = locked_tree_rows
        .iter()
        .map(|row| (row.id, row.parent_run_id))
        .collect::<std::collections::HashMap<_, _>>();
    let locked_visited = locked_tree_rows
        .iter()
        .filter(|row| {
            let mut current = Some(row.id);
            let mut seen = HashSet::new();
            while let Some(run_id) = current {
                if run_id == root_run_id {
                    return true;
                }
                if !seen.insert(run_id) {
                    return false;
                }
                current = locked_parent_by_run.get(&run_id).copied().flatten();
            }
            false
        })
        .map(|row| row.id)
        .collect::<HashSet<_>>();
    let locked_thread_ids = locked_tree_rows
        .iter()
        .map(|row| row.thread_id)
        .collect::<HashSet<_>>();
    if locked_visited != visited || !locked_thread_ids.is_subset(&lock_thread_ids) {
        tx.rollback().await?;
        return Box::pin(cancel_run_tree(db, root_run_id, error_message)).await;
    }
    let mut lock_run_ids = visited.clone();
    lock_run_ids.extend(boundary_parent_run_ids.iter().copied());
    let locked_runs = lock_runs_on(&tx, lock_run_ids.iter().copied()).await?;
    let mut rows = locked_runs
        .iter()
        .filter(|row| visited.contains(&row.id))
        .cloned()
        .collect::<Vec<_>>();
    let mut boundary_parent_runs = locked_runs
        .iter()
        .filter(|row| boundary_parent_run_ids.contains(&row.id))
        .map(|row| (row.id, row.clone()))
        .collect::<std::collections::HashMap<_, _>>();
    let _locked_turns = lock_turns_on(&tx, locked_runs.iter().map(|row| row.turn_id)).await?;
    let locked_delegation_ids = delegation::Entity::find()
        .select_only()
        .column(delegation::Column::Id)
        .filter(
            sea_orm::Condition::any()
                .add(delegation::Column::ParentRunId.is_in(visited.iter().copied()))
                .add(delegation::Column::ChildRunId.is_in(visited.iter().copied())),
        )
        .into_tuple::<i64>()
        .all(&tx)
        .await?;
    if locked_delegation_ids
        .iter()
        .copied()
        .collect::<HashSet<_>>()
        != delegation_ids.iter().copied().collect::<HashSet<_>>()
    {
        tx.rollback().await?;
        return Box::pin(cancel_run_tree(db, root_run_id, error_message)).await;
    }
    let locked_delegations = lock_delegations_on(&tx, locked_delegation_ids).await?;
    if !rows.iter().any(|row| row.id == root.id) {
        return Err(AgentDbError::NotFound(format!(
            "Agent run 不存在: {root_run_id}"
        )));
    }
    let timestamp = now();
    let mut cancelled = Vec::with_capacity(rows.len());
    let mut turn_ids = HashSet::new();
    let mut tree_run_ids = Vec::with_capacity(rows.len());
    for row in rows.drain(..) {
        turn_ids.insert(row.turn_id);
        tree_run_ids.push(row.id);
        if is_run_terminal(&row.status) || !row.is_active {
            cancelled.push(row);
            continue;
        }
        let mut active: run::ActiveModel = row.into();
        active.status = Set("cancelled".to_string());
        active.waiting_reason = Set(None);
        active.is_active = Set(false);
        active.last_error = Set(error_message.map(str::to_string));
        active.worker_id = Set(None);
        active.lease_token = Set(None);
        active.lease_expires_at = Set(None);
        active.updated_at = Set(timestamp);
        active.completed_at = Set(Some(timestamp));
        let updated = active.update(&tx).await?;
        append_run_event_on(
            &tx,
            &updated,
            "run_cancelled",
            "cancelled",
            "Agent run 已取消",
            None,
            None,
            None,
            error_message.map(|error| json!({ "error": error })),
        )
        .await?;
        cancelled.push(updated);
    }

    if !turn_ids.is_empty() {
        turn::Entity::update_many()
            .col_expr(
                turn::Column::Status,
                sea_orm::sea_query::Expr::value("cancelled"),
            )
            .col_expr(
                turn::Column::ErrorMessage,
                sea_orm::sea_query::Expr::value(error_message.map(str::to_string)),
            )
            .col_expr(
                turn::Column::UpdatedAt,
                sea_orm::sea_query::Expr::value(timestamp),
            )
            .col_expr(
                turn::Column::CompletedAt,
                sea_orm::sea_query::Expr::value(Some(timestamp)),
            )
            .filter(turn::Column::Id.is_in(turn_ids.iter().copied()))
            .filter(turn::Column::CompletedAt.is_null())
            .exec(&tx)
            .await?;
    }
    if !tree_run_ids.is_empty() {
        terminate_executions_on(
            &tx,
            tree_run_ids.iter().copied(),
            ExecutionTermination::Cancelled,
            error_message,
            timestamp,
        )
        .await?;
        interrupt_streaming_messages_on(&tx, tree_run_ids.iter().copied(), error_message).await?;
        for row in locked_delegations {
            if is_delegation_terminal(&row.status) {
                continue;
            }
            let boundary_child_run_id = row.child_run_id.filter(|child_run_id| {
                visited.contains(child_run_id) && !visited.contains(&row.parent_run_id)
            });
            let boundary_child_run = boundary_child_run_id
                .and_then(|child_run_id| cancelled.iter().find(|run| run.id == child_run_id));
            // Cancelling an already-terminal child is an idempotent no-op. In
            // particular, do not turn a completed child awaiting parent
            // acceptance into a cancelled delegation.
            if boundary_child_run_id.is_some()
                && boundary_child_run.is_none_or(|run| run.status != "cancelled")
            {
                continue;
            }
            let mut active: delegation::ActiveModel = row.into();
            active.status = Set("cancelled".to_string());
            active.error_message = Set(error_message.map(str::to_string));
            active.updated_at = Set(timestamp);
            active.completed_at = Set(Some(timestamp));
            let delegation = active.update(&tx).await?;

            let Some(child_run) = boundary_child_run else {
                continue;
            };
            let parent_run = boundary_parent_runs
                .get(&delegation.parent_run_id)
                .cloned()
                .ok_or_else(|| {
                    AgentDbError::Conflict(format!(
                        "Agent child cancellation 缺少 parent run: {}",
                        delegation.parent_run_id
                    ))
                })?;
            if !parent_run.is_active || is_run_terminal(&parent_run.status) {
                return Err(AgentDbError::Conflict(format!(
                    "Agent child cancellation 无法投递给已终止 parent run: {}",
                    parent_run.id
                )));
            }
            if child_run.thread_id != delegation.child_thread_id
                || parent_run.thread_id != delegation.parent_thread_id
            {
                return Err(AgentDbError::Conflict(format!(
                    "Agent child cancellation delegation lineage 不一致: {}",
                    delegation.id
                )));
            }

            let reason = error_message
                .unwrap_or("Agent child run 已取消")
                .to_string();
            let parent_tool_message = if let Some(execution_id) = delegation.parent_execution_id {
                let parent_execution = execution::Entity::find_by_id(execution_id)
                    .lock_exclusive()
                    .one(&tx)
                    .await?
                    .ok_or_else(|| {
                        AgentDbError::NotFound(format!(
                            "Agent parent delegation execution 不存在: {execution_id}"
                        ))
                    })?;
                let result = json!({
                    "delegation_id": delegation.id.to_string(),
                    "child_run_id": child_run.id.to_string(),
                    "cancelled": true,
                    "reason": reason.clone(),
                });
                let message = append_parent_delegation_tool_message_on(
                    &tx,
                    &parent_run,
                    child_run,
                    &delegation,
                    &parent_execution,
                    "cancelled",
                    result.clone(),
                )
                .await?;
                finish_parent_delegation_execution_on(
                    &tx,
                    parent_execution,
                    "cancelled",
                    result,
                    "Child Agent 已取消".into(),
                    None,
                    Some(reason.clone()),
                    timestamp,
                )
                .await?;
                Some(message)
            } else {
                None
            };

            let mut inbox_payload = json!({
                "delegation_id": delegation.id.to_string(),
                "child_run_id": child_run.id.to_string(),
                "cancelled": true,
                "reason": reason.clone(),
            });
            if let Some(message) = parent_tool_message.as_ref() {
                inbox_payload["message_id"] = json!(message.id.to_string());
            }
            enqueue_inbox(
                &tx,
                AgentInboxInput {
                    thread_id: parent_run.thread_id,
                    turn_id: Some(parent_run.turn_id),
                    run_id: Some(parent_run.id),
                    source_kind: "subagent".into(),
                    source_thread_id: Some(child_run.thread_id),
                    inbox_kind: "child_result".into(),
                    delivery_mode: "followup".into(),
                    priority: 20,
                    dedupe_key: format!("child-cancelled:{}", delegation.id),
                    content: format!("Child Agent cancelled: {reason}"),
                    payload_json: Some(inbox_payload),
                    available_at: None,
                },
            )
            .await?;

            if parent_run.status == "waiting"
                && parent_run.waiting_reason.as_deref() == Some("child")
            {
                let mut parent_active: run::ActiveModel = parent_run.into();
                parent_active.status = Set("queued".to_string());
                parent_active.waiting_reason = Set(None);
                parent_active.worker_id = Set(None);
                parent_active.lease_token = Set(None);
                parent_active.lease_expires_at = Set(None);
                parent_active.next_attempt_at = Set(timestamp);
                parent_active.updated_at = Set(timestamp);
                let parent_run = parent_active.update(&tx).await?;
                boundary_parent_runs.insert(parent_run.id, parent_run);
            }
        }
        approval::Entity::update_many()
            .col_expr(
                approval::Column::Status,
                sea_orm::sea_query::Expr::value("rejected"),
            )
            .col_expr(
                approval::Column::DecisionNote,
                sea_orm::sea_query::Expr::value(Some("Agent run 已取消".to_string())),
            )
            .col_expr(
                approval::Column::DecidedAt,
                sea_orm::sea_query::Expr::value(Some(timestamp)),
            )
            .filter(approval::Column::RunId.is_in(tree_run_ids.iter().copied()))
            .filter(approval::Column::Status.eq("pending"))
            .exec(&tx)
            .await?;
        inbox::Entity::update_many()
            .col_expr(
                inbox::Column::Status,
                sea_orm::sea_query::Expr::value("cancelled"),
            )
            .col_expr(
                inbox::Column::ClaimToken,
                sea_orm::sea_query::Expr::value(Option::<String>::None),
            )
            .col_expr(
                inbox::Column::ClaimExpiresAt,
                sea_orm::sea_query::Expr::value(Option::<NaiveDateTime>::None),
            )
            .col_expr(
                inbox::Column::ConsumedAt,
                sea_orm::sea_query::Expr::value(Some(timestamp)),
            )
            .filter(inbox::Column::RunId.is_in(tree_run_ids.iter().copied()))
            .filter(inbox::Column::Status.is_in(["pending", "claimed"]))
            .exec(&tx)
            .await?;
    }
    for thread_id in lock_thread_ids {
        let _ = refresh_thread_status_on(&tx, thread_id).await?;
    }
    tx.commit().await?;
    Ok(cancelled)
}

async fn complete_turn<C>(
    db: &C,
    turn_id: i64,
    status: &str,
    final_message_id: Option<i64>,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
) -> Result<turn::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_turn_status(status)?;
    let row = turn::Entity::find_by_id(turn_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {turn_id}")))?;
    if is_turn_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent turn 已终止，不能从 {} 变为 {status}: {turn_id}",
            row.status
        )));
    }
    let timestamp = now();
    let mut active: turn::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.final_message_id = Set(final_message_id);
    active.result_refs_json = Set(result_refs_json);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    if is_turn_terminal(status) {
        active.completed_at = Set(Some(timestamp));
    } else {
        active.completed_at = Set(None);
    }
    active.update(db).await.map_err(AgentDbError::Database)
}

/// Marks a Turn active while fencing the transition to its Run lease.  The
/// check and Turn update share one transaction, so a recovered/stale worker
/// cannot resurrect a queued Turn after its Run lease has changed hands.
pub async fn mark_turn_active_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
) -> Result<turn::Model, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let turn_row = turn::Entity::find_by_id(run_row.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
    if turn_row.thread_id != run_row.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent turn 与 run 不一致: {run_id}"
        )));
    }
    if is_turn_terminal(&turn_row.status) {
        return Err(AgentDbError::Conflict(format!(
            "Agent turn 已终止，不能激活: {}",
            turn_row.id
        )));
    }
    let timestamp = now();
    let mut active: turn::ActiveModel = turn_row.into();
    active.status = Set("active".to_string());
    active.updated_at = Set(timestamp);
    active.completed_at = Set(None);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

/// Atomically appends an assistant final message and closes its Turn/Run.
///
/// The lease is checked while the Run row is locked, so a stale worker cannot
/// leave a completed message behind while the durable Run remains active (or
/// vice versa).
pub async fn finish_run_with_message(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    message: AgentMessageInput,
    result_refs_json: Option<Value>,
    observed_input_message_id: i64,
) -> Result<Option<(run::Model, message::Model)>, AgentDbError> {
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    if message.thread_id != run_row.thread_id
        || message.turn_id != Some(run_row.turn_id)
        || message.run_id != Some(run_id)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent final message 与 run 不一致: {run_id}"
        )));
    }
    if requeue_for_new_input_on(&tx, &run_row, observed_input_message_id).await? {
        tx.commit().await?;
        return Ok(None);
    }
    turn::Entity::find_by_id(run_row.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent turn 不存在: {}", run_row.turn_id)))?;
    let final_message = if let Some(streaming) = message::Entity::find()
        .filter(message::Column::RunId.eq(run_row.id))
        .filter(message::Column::Role.eq("assistant"))
        .filter(message::Column::Status.eq("streaming"))
        .order_by_desc(message::Column::Id)
        .lock_exclusive()
        .one(&tx)
        .await?
    {
        let mut active: message::ActiveModel = streaming.into();
        active.content_sha256 = Set(sha256_hex(&message.content));
        active.content = Set(message.content);
        active.status = Set("complete".into());
        active.payload_json = Set(message.payload_json);
        active.updated_at = Set(timestamp);
        active.update(&tx).await?
    } else {
        append_message_on(&tx, message).await?
    };
    append_message_event_on(&tx, &final_message, "message_completed", "complete").await?;
    let _turn = complete_turn(
        &tx,
        run_row.turn_id,
        "completed",
        Some(final_message.id),
        result_refs_json,
        None,
    )
    .await?;
    let mut active: run::ActiveModel = run_row.into();
    active.status = Set("completed".to_string());
    active.waiting_reason = Set(None);
    active.is_active = Set(false);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(Some(timestamp));
    active.worker_id = Set(None);
    active.lease_token = Set(None);
    active.lease_expires_at = Set(None);
    let completed_run = active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &completed_run,
        "run_completed",
        "completed",
        "Agent run 已完成",
        None,
        None,
        Some(final_message.id),
        None,
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, completed_run.thread_id).await?;
    tx.commit().await?;
    Ok(Some((completed_run, final_message)))
}

// The sender and finalizer both hold the Thread lock. An accepted follow-up
// either belongs to this still-active Run or starts a new Turn after it closes.
async fn requeue_for_new_input_on<C: ConnectionTrait>(
    db: &C,
    row: &run::Model,
    observed_input_message_id: i64,
) -> Result<bool, AgentDbError> {
    if message::Entity::find()
        .filter(message::Column::RunId.eq(row.id))
        .filter(message::Column::Role.eq("user"))
        .filter(message::Column::Id.gt(observed_input_message_id))
        .one(db)
        .await?
        .is_none()
    {
        return Ok(false);
    }
    update_run_state_on(db, row.id, row.lease_token.as_deref(), "queued", None, None).await?;
    refresh_thread_status_on(db, row.thread_id).await?;
    Ok(true)
}

async fn consume_observed_user_inbox_on<C: ConnectionTrait>(
    db: &C,
    owner: &run::Model,
    message_id: i64,
) -> Result<(), AgentDbError> {
    let rows = inbox::Entity::find()
        .filter(inbox::Column::RunId.eq(owner.id))
        .filter(inbox::Column::SourceKind.eq("user"))
        .filter(inbox::Column::Status.is_in(["pending", "claimed"]))
        .lock_exclusive()
        .all(db)
        .await?;
    for row in rows {
        let id = row
            .payload_json
            .as_ref()
            .and_then(|v| v.get("message_id"))
            .and_then(Value::as_str)
            .and_then(|v| v.parse::<i64>().ok())
            .ok_or_else(|| AgentDbError::InvalidArgument("用户 inbox 缺少 message_id".into()))?;
        if id <= message_id {
            let mut active: inbox::ActiveModel = row.into();
            active.status = Set("consumed".into());
            active.claim_token = Set(None);
            active.claim_expires_at = Set(None);
            active.consumed_at = Set(Some(now()));
            active.update(db).await?;
        }
    }
    Ok(())
}

pub async fn update_run_usage(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    usage_json: Value,
) -> Result<run::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&row, lease_token, timestamp)?;
    // The process-local snapshot requirement is an admission invariant. A
    // Worker usage writes must not be able to erase it by sending a stale or
    // hand-built usage object, otherwise a later resume could run without the
    // credentials/configuration it was admitted with.
    let mut usage_json = usage_json;
    let snapshot_required = match row.usage_json.get("runtime_snapshot_required") {
        Some(Value::Bool(required)) => *required,
        Some(_) => {
            return Err(AgentDbError::InvalidArgument(format!(
                "Agent run {run_id} runtime_snapshot_required 必须是 boolean"
            )));
        }
        None => {
            return Err(AgentDbError::InvalidArgument(format!(
                "Agent run {run_id} 缺少 runtime_snapshot_required 标记"
            )));
        }
    };
    let usage_object = usage_json
        .as_object_mut()
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent run usage_json 必须是对象".into()))?;
    usage_object.insert(
        "runtime_snapshot_required".into(),
        Value::Bool(snapshot_required),
    );
    let mut active: run::ActiveModel = row.into();
    active.usage_json = Set(usage_json);
    active.updated_at = Set(timestamp);
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

async fn create_execution_on<C>(
    db: &C,
    input: AgentExecutionInput,
    run_row: run::Model,
) -> Result<execution::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_execution_status(&input.status)?;
    if input.execution_key.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "execution_key 不能为空".into(),
        ));
    }
    let args_digest = input.arguments_json.as_ref().map(json_sha256).transpose()?;
    if let Some(existing) = execution::Entity::find()
        .filter(execution::Column::RunId.eq(input.run_id))
        .filter(execution::Column::ExecutionKey.eq(&input.execution_key))
        .one(db)
        .await?
    {
        if existing.arguments_sha256 != args_digest
            || existing.execution_kind != input.execution_kind
            || existing.tool_name != input.tool_name
            || existing.effect != input.effect
            || existing.replay_policy != input.replay_policy
            || existing.resource_preconditions_json != input.resource_preconditions_json
        {
            return Err(AgentDbError::Conflict(format!(
                "execution_key 已用于不同操作、参数或执行策略: {}",
                input.execution_key
            )));
        }
        return Ok(existing);
    }
    let sequence = run_row
        .next_execution_sequence
        .checked_add(1)
        .ok_or_else(|| AgentDbError::InvalidArgument("Agent execution sequence 溢出".into()))?;
    let timestamp = now();
    let row = execution::ActiveModel {
        run_id: Set(input.run_id),
        sequence: Set(sequence),
        execution_key: Set(input.execution_key),
        execution_kind: Set(input.execution_kind),
        status: Set(input.status),
        tool_name: Set(input.tool_name),
        effect: Set(input.effect),
        replay_policy: Set(input.replay_policy),
        arguments_json: Set(input.arguments_json),
        arguments_sha256: Set(args_digest),
        resource_preconditions_json: Set(input.resource_preconditions_json),
        result_json: Set(None),
        result_summary: Set(None),
        result_refs_json: Set(None),
        error_message: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        started_at: Set(None),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await?;
    let mut active: run::ActiveModel = run_row.into();
    active.next_execution_sequence = Set(sequence);
    active.updated_at = Set(timestamp);
    active.update(db).await?;
    Ok(row)
}

/// Creates an execution while fencing it to the currently-held Run lease.
///
/// The Run row is locked and the token/expiry are checked in the same
/// transaction as the execution insert, so an expired worker cannot enqueue a
/// side effect after another worker has taken over recovery.
pub async fn create_execution_for_run_lease(
    db: &DatabaseConnection,
    input: AgentExecutionInput,
    lease_token: &str,
) -> Result<execution::Model, AgentDbError> {
    if input.status != "prepared" {
        return Err(AgentDbError::InvalidArgument(
            "worker 只能创建 prepared Agent execution".into(),
        ));
    }
    let tx = db.begin().await?;
    budget::reserve_budget_on(&tx, &input).await?;
    let run_row = run::Entity::find_by_id(input.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound("Agent run 不存在".into()))?;
    validate_live_run_lease(&run_row, lease_token, now())?;
    let row = create_execution_on(&tx, input, run_row).await?;
    tx.commit().await?;
    Ok(row)
}

/// Updates an execution while fencing the write to the current Run lease.
///
/// Execution results are durable side-effect evidence. A worker whose lease
/// expired must not be able to overwrite the result after another worker has
/// taken over recovery.
pub async fn update_execution_for_run_lease(
    db: &DatabaseConnection,
    run_id: i64,
    lease_token: &str,
    execution_id: i64,
    status: &str,
    result_json: Option<Value>,
    result_summary: Option<String>,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
) -> Result<execution::Model, AgentDbError> {
    validate_execution_status(status)?;
    let identity = run::Entity::find_by_id(run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {run_id}")))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    let row = execution::Entity::find_by_id(execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent execution 不存在: {execution_id}")))?;
    if row.run_id != run_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 不属于 run {run_id}: {execution_id}"
        )));
    }
    if is_execution_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 已终止，不能从 {} 变为 {status}: {execution_id}",
            row.status
        )));
    }
    if row.status == "awaiting_user" && status != "dispatching" {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 获批后必须先进入 dispatching: {execution_id}"
        )));
    }
    let approval_row = approval::Entity::find()
        .filter(approval::Column::ExecutionId.eq(execution_id))
        .lock_exclusive()
        .one(&tx)
        .await?;
    if row.status == "awaiting_user" {
        let approval = approval_row.as_ref().ok_or_else(|| {
            AgentDbError::Conflict(format!("Agent execution 缺少 approval: {execution_id}"))
        })?;
        if approval.status != "approved" {
            return Err(AgentDbError::Conflict(format!(
                "Agent execution 未获批: {execution_id} ({})",
                approval.status
            )));
        }
    }
    // Approval is evidence until the side effect has actually succeeded.
    // Consuming it while entering `dispatching` loses the authorization when
    // a provider fails and makes a retry appear unapproved.
    if status == "succeeded"
        && let Some(approval) = approval_row
    {
        if approval.status != "approved" {
            return Err(AgentDbError::Conflict(format!(
                "Agent execution approval 不可消费: {execution_id} ({})",
                approval.status
            )));
        }
        let mut active: approval::ActiveModel = approval.into();
        active.status = Set("consumed".to_string());
        active.consumed_at = Set(Some(timestamp));
        active.update(&tx).await?;
    }
    let terminal = is_execution_terminal(status);
    let mut active: execution::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.result_json = Set(result_json);
    active.result_summary = Set(result_summary);
    active.result_refs_json = Set(result_refs_json);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    active.completed_at = Set(terminal.then_some(timestamp));
    let row = active.update(&tx).await?;
    tx.commit().await?;
    Ok(row)
}

async fn create_approval<C>(
    db: &C,
    input: AgentApprovalInput,
) -> Result<approval::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let timestamp = now();
    approval::ActiveModel {
        thread_id: Set(input.thread_id),
        run_id: Set(input.run_id),
        execution_id: Set(input.execution_id),
        operation_digest: Set(input.operation_digest),
        status: Set("pending".to_string()),
        operation_json: Set(input.operation_json),
        resource_preconditions_json: Set(input.resource_preconditions_json),
        decided_by_user_id: Set(None),
        decision_note: Set(None),
        expires_at: Set(input.expires_at),
        created_at: Set(timestamp),
        decided_at: Set(None),
        consumed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::Database)
}

/// Atomically requests user approval for a prepared write execution.
///
/// Registering the approval, moving the Execution to `awaiting_user`, and
/// releasing the Run lease into `waiting(approval)` are one durable decision.
/// This prevents an approval response from arriving between separate commits
/// and being lost before the worker starts waiting.
pub async fn request_approval_for_run_lease(
    db: &DatabaseConnection,
    lease_token: &str,
    input: AgentApprovalInput,
) -> Result<(approval::Model, execution::Model, run::Model), AgentDbError> {
    if input.operation_digest.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent approval operation_digest 不能为空".into(),
        ));
    }
    let identity = run::Entity::find_by_id(input.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", input.run_id)))?;
    if identity.thread_id != input.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 与 run thread 不一致: {}",
            input.run_id
        )));
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(input.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", input.run_id)))?;
    let timestamp = now();
    validate_live_run_lease(&run_row, lease_token, timestamp)?;
    let execution_row = execution::Entity::find_by_id(input.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent execution 不存在: {}", input.execution_id))
        })?;
    if execution_row.run_id != run_row.id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval execution 不属于 run {}: {}",
            run_row.id, execution_row.id
        )));
    }
    if execution_row.status != "prepared" {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 不能请求审批: {} ({})",
            execution_row.id, execution_row.status
        )));
    }
    if approval::Entity::find()
        .filter(approval::Column::ExecutionId.eq(execution_row.id))
        .lock_exclusive()
        .one(&tx)
        .await?
        .is_some()
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent execution 已存在 approval: {}",
            execution_row.id
        )));
    }

    let approval_row = create_approval(&tx, input).await?;
    let mut execution_active: execution::ActiveModel = execution_row.into();
    execution_active.status = Set("awaiting_user".to_string());
    execution_active.updated_at = Set(timestamp);
    let execution_row = execution_active.update(&tx).await?;

    let mut run_active: run::ActiveModel = run_row.into();
    run_active.status = Set("waiting".to_string());
    run_active.waiting_reason = Set(Some("approval".to_string()));
    run_active.worker_id = Set(None);
    run_active.lease_token = Set(None);
    run_active.lease_expires_at = Set(None);
    run_active.updated_at = Set(timestamp);
    let run_row = run_active.update(&tx).await?;
    append_run_event_on(
        &tx,
        &run_row,
        "approval_requested",
        "pending",
        "Agent 操作等待用户审批",
        Some(execution_row.id),
        None,
        None,
        Some(json!({ "approval_id": approval_row.id.to_string() })),
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    tx.commit().await?;
    Ok((approval_row, execution_row, run_row))
}

/// Resolves an approval while holding the related Run/Execution locks.
/// Approved operations are left in `awaiting_user` for the worker to dispatch;
/// rejected operations are terminally rejected.  A waiting parent Run is
/// requeued in either case so it can make the next model decision.
pub async fn resolve_approval_for_run(
    db: &DatabaseConnection,
    approval_id: i64,
    user_id: i64,
    approved: bool,
    reason: Option<String>,
) -> Result<approval::Model, AgentDbError> {
    if user_id <= 0 {
        return Err(AgentDbError::InvalidArgument("user_id 无效".into()));
    }
    let identity = approval::Entity::find_by_id(approval_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    let run_identity = run::Entity::find_by_id(identity.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    if identity.thread_id != run_identity.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联记录不一致: {approval_id}"
        )));
    }
    if identity
        .expires_at
        .is_some_and(|expires_at| expires_at <= now())
    {
        return expire_pending_approval(db, approval_id)
            .await?
            .ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent approval 已由其他请求处理: {approval_id}"))
            });
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(run_identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", run_identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(identity.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    let execution_row = execution::Entity::find_by_id(identity.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent execution 不存在: {}", identity.execution_id))
        })?;
    let approval_row = approval::Entity::find_by_id(approval_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    if approval_row.status != "pending" {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 已处理: {approval_id}"
        )));
    }
    let timestamp = now();
    if approval_row
        .expires_at
        .is_some_and(|expires_at| expires_at <= timestamp)
    {
        tx.rollback().await?;
        return expire_pending_approval(db, approval_id)
            .await?
            .ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent approval 已由其他请求处理: {approval_id}"))
            });
    }
    if execution_row.run_id != run_row.id || approval_row.thread_id != run_row.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联记录不一致: {approval_id}"
        )));
    }
    if !run_row.is_active {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 所属 run 已终止: {}",
            run_row.id
        )));
    }
    if execution_row.status != "awaiting_user" {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval execution 不在 awaiting_user: {} ({})",
            execution_row.id, execution_row.status
        )));
    }
    let mut approval_active: approval::ActiveModel = approval_row.into();
    approval_active.status = Set(if approved { "approved" } else { "rejected" }.to_string());
    approval_active.decided_by_user_id = Set(Some(user_id));
    approval_active.decision_note = Set(reason);
    approval_active.decided_at = Set(Some(timestamp));
    let approval_row = approval_active.update(&tx).await?;

    if !approved {
        let mut execution_active: execution::ActiveModel = execution_row.into();
        execution_active.status = Set("rejected".to_string());
        execution_active.error_message = Set(Some("用户拒绝 Agent 操作".to_string()));
        execution_active.updated_at = Set(timestamp);
        execution_active.completed_at = Set(Some(timestamp));
        execution_active.update(&tx).await?;
    }
    let wake_parent =
        run_row.status == "waiting" && run_row.waiting_reason.as_deref() == Some("approval");
    let mut run_active: run::ActiveModel = run_row.into();
    if wake_parent {
        run_active.status = Set("queued".to_string());
        run_active.waiting_reason = Set(None);
        run_active.worker_id = Set(None);
        run_active.lease_token = Set(None);
        run_active.lease_expires_at = Set(None);
        run_active.next_attempt_at = Set(timestamp);
        run_active.updated_at = Set(timestamp);
        let run_row = run_active.update(&tx).await?;
        let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    }
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: run_identity.thread_id,
            turn_id: Some(run_identity.turn_id),
            run_id: Some(run_identity.id),
            execution_id: Some(identity.execution_id),
            delegation_id: None,
            message_id: None,
            actor_kind: "user".into(),
            actor_id: Some(user_id.to_string()),
            kind: "approval_resolved".into(),
            status: approval_row.status.clone(),
            public_text: if approved {
                "用户已批准 Agent 操作".into()
            } else {
                "用户已拒绝 Agent 操作".into()
            },
            payload_json: Some(json!({
                "approval_id": approval_row.id.to_string(),
                "reason": approval_row.decision_note.clone(),
            })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(approval_row)
}

/// Expires pending approvals whose deadline has passed.  Expiry is processed
/// as a durable workflow transition so a run cannot remain indefinitely in
/// `waiting(approval)` when nobody opens the approval dialog.
pub async fn expire_pending_approvals(
    db: &DatabaseConnection,
    limit: u64,
) -> Result<Vec<approval::Model>, AgentDbError> {
    if limit == 0 {
        return Err(AgentDbError::InvalidArgument(
            "approval expiry limit 必须大于 0".into(),
        ));
    }
    let ids = approval::Entity::find()
        .select_only()
        .column(approval::Column::Id)
        .filter(approval::Column::Status.eq("pending"))
        .filter(approval::Column::ExpiresAt.is_not_null())
        .filter(approval::Column::ExpiresAt.lte(now()))
        .order_by_asc(approval::Column::ExpiresAt)
        .order_by_asc(approval::Column::Id)
        .limit(limit)
        .into_tuple::<i64>()
        .all(db)
        .await?;
    let mut expired = Vec::with_capacity(ids.len());
    for id in ids {
        if let Some(row) = expire_pending_approval(db, id).await? {
            expired.push(row);
        }
    }
    Ok(expired)
}

/// Attempts one expiry transition.  `None` means another resolver won the
/// race or the deadline was extended before the row was locked.
async fn expire_pending_approval(
    db: &DatabaseConnection,
    approval_id: i64,
) -> Result<Option<approval::Model>, AgentDbError> {
    let identity = approval::Entity::find_by_id(approval_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    let run_identity = run::Entity::find_by_id(identity.run_id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    if identity.thread_id != run_identity.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联记录不一致: {approval_id}"
        )));
    }
    let tx = db.begin().await?;
    thread::Entity::find_by_id(run_identity.thread_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent thread 不存在: {}", run_identity.thread_id))
        })?;
    let run_row = run::Entity::find_by_id(identity.run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent run 不存在: {}", identity.run_id)))?;
    let execution_row = execution::Entity::find_by_id(identity.execution_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent execution 不存在: {}", identity.execution_id))
        })?;
    let approval_row = approval::Entity::find_by_id(approval_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent approval 不存在: {approval_id}")))?;
    let timestamp = now();
    if approval_row.status != "pending"
        || approval_row
            .expires_at
            .is_none_or(|expires_at| expires_at > timestamp)
    {
        tx.commit().await?;
        return Ok(None);
    }
    if execution_row.run_id != run_row.id
        || approval_row.thread_id != run_row.thread_id
        || execution_row.status != "awaiting_user"
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent approval 关联 execution 状态不一致: {approval_id}"
        )));
    }
    let mut approval_active: approval::ActiveModel = approval_row.into();
    approval_active.status = Set("expired".to_string());
    approval_active.decision_note = Set(Some("Agent approval 已过期".to_string()));
    approval_active.decided_at = Set(Some(timestamp));
    let expired = approval_active.update(&tx).await?;

    let mut execution_active: execution::ActiveModel = execution_row.into();
    execution_active.status = Set("rejected".to_string());
    execution_active.error_message = Set(Some("Agent approval 已过期".to_string()));
    execution_active.updated_at = Set(timestamp);
    execution_active.completed_at = Set(Some(timestamp));
    execution_active.update(&tx).await?;

    if run_row.is_active
        && run_row.status == "waiting"
        && run_row.waiting_reason.as_deref() == Some("approval")
    {
        let mut run_active: run::ActiveModel = run_row.into();
        run_active.status = Set("queued".to_string());
        run_active.waiting_reason = Set(None);
        run_active.worker_id = Set(None);
        run_active.lease_token = Set(None);
        run_active.lease_expires_at = Set(None);
        run_active.next_attempt_at = Set(timestamp);
        run_active.updated_at = Set(timestamp);
        let run_row = run_active.update(&tx).await?;
        let _ = refresh_thread_status_on(&tx, run_row.thread_id).await?;
    } else {
        let _ = refresh_thread_status_on(&tx, run_identity.thread_id).await?;
    }
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: run_identity.thread_id,
            turn_id: Some(run_identity.turn_id),
            run_id: Some(run_identity.id),
            execution_id: Some(identity.execution_id),
            delegation_id: None,
            message_id: None,
            actor_kind: "system".into(),
            actor_id: None,
            kind: "approval_expired".into(),
            status: "expired".into(),
            public_text: "Agent 操作审批已过期".into(),
            payload_json: Some(json!({ "approval_id": expired.id.to_string() })),
        },
    )
    .await?;
    tx.commit().await?;
    Ok(Some(expired))
}

pub async fn create_delegation<C>(
    db: &C,
    input: AgentDelegationInput,
) -> Result<delegation::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_delegation_status(&input.status)?;
    if input.delegation_key.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "delegation_key 不能为空".into(),
        ));
    }
    if input.capability.trim().is_empty() || input.objective.trim().is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "delegation capability/objective 不能为空".into(),
        ));
    }
    if input.parent_thread_id == input.child_thread_id {
        return Err(AgentDbError::InvalidArgument(
            "delegation 不能委派给同一个 thread".into(),
        ));
    }
    if input.child_turn_id.is_some() || input.child_run_id.is_some() {
        return Err(AgentDbError::InvalidArgument(
            "delegation child 必须通过 bind_delegation_child 原子绑定".into(),
        ));
    }
    let timestamp = now();
    delegation::ActiveModel {
        parent_thread_id: Set(input.parent_thread_id),
        parent_run_id: Set(input.parent_run_id),
        parent_execution_id: Set(input.parent_execution_id),
        child_thread_id: Set(input.child_thread_id),
        child_turn_id: Set(input.child_turn_id),
        child_run_id: Set(input.child_run_id),
        delegation_key: Set(input.delegation_key),
        capability: Set(input.capability),
        objective: Set(input.objective),
        success_criteria_json: Set(input.success_criteria_json),
        scope_json: Set(input.scope_json),
        budget_json: Set(input.budget_json),
        status: Set(input.status),
        result_refs_json: Set(None),
        result_summary: Set(None),
        error_message: Set(None),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        completed_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::Database)
}

/// Replaying a delegation key is only idempotent when it describes the same
/// business operation.  A reused key with different parameters would
/// otherwise silently return the original child and make the caller believe
/// its new request was accepted.
fn validate_delegation_replay(
    existing: &delegation::Model,
    requested: &AgentDelegationInput,
) -> Result<(), AgentDbError> {
    let matches = existing.parent_thread_id == requested.parent_thread_id
        && existing.parent_run_id == requested.parent_run_id
        && existing.parent_execution_id == requested.parent_execution_id
        && existing.child_thread_id == requested.child_thread_id
        && existing.capability.trim() == requested.capability.trim()
        && existing.objective.trim() == requested.objective.trim()
        && existing.success_criteria_json == requested.success_criteria_json
        && existing.scope_json == requested.scope_json
        && existing.budget_json == requested.budget_json;
    if matches {
        Ok(())
    } else {
        Err(AgentDbError::Conflict(format!(
            "Agent delegation_key 已绑定不同请求: {}",
            requested.delegation_key
        )))
    }
}

pub async fn update_delegation<C>(
    db: &C,
    id: i64,
    status: &str,
    result_refs_json: Option<Value>,
    result_summary: Option<String>,
    error_message: Option<String>,
) -> Result<delegation::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    validate_delegation_status(status)?;
    let row = delegation::Entity::find_by_id(id)
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent delegation 不存在: {id}")))?;
    if is_delegation_terminal(&row.status) && row.status != status {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 已终止，不能从 {} 变为 {status}: {id}",
            row.status
        )));
    }
    let terminal = is_delegation_terminal(status);
    let timestamp = now();
    let mut active: delegation::ActiveModel = row.into();
    active.status = Set(status.to_string());
    active.result_refs_json = Set(result_refs_json);
    active.result_summary = Set(result_summary);
    active.error_message = Set(error_message);
    active.updated_at = Set(timestamp);
    if terminal {
        active.completed_at = Set(Some(timestamp));
    } else {
        active.completed_at = Set(None);
    }
    active.update(db).await.map_err(AgentDbError::Database)
}

pub async fn bind_delegation_child(
    db: &DatabaseConnection,
    id: i64,
    parent_run_id: i64,
    child_turn_id: i64,
    child_run_id: i64,
) -> Result<delegation::Model, AgentDbError> {
    let tx = db.begin().await?;
    let row = bind_delegation_child_on(&tx, id, parent_run_id, child_turn_id, child_run_id).await?;
    tx.commit().await?;
    Ok(row)
}

async fn bind_delegation_child_on<C>(
    db: &C,
    id: i64,
    parent_run_id: i64,
    child_turn_id: i64,
    child_run_id: i64,
) -> Result<delegation::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    let parent_run = run::Entity::find_by_id(parent_run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent parent run 不存在: {parent_run_id}"))
        })?;
    let row = delegation::Entity::find_by_id(id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent delegation 不存在: {id}")))?;
    if row.parent_run_id != parent_run_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 不属于 parent run {parent_run_id}: {id}"
        )));
    }
    if parent_run.thread_id != row.parent_thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent run 与 delegation 绑定不一致: {id}"
        )));
    }
    if row.child_turn_id == Some(child_turn_id) && row.child_run_id == Some(child_run_id) {
        let child_run = run::Entity::find_by_id(child_run_id)
            .lock_exclusive()
            .one(db)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}"))
            })?;
        let expected_root_run_id = parent_run.root_run_id.unwrap_or(parent_run.id);
        if child_run.thread_id != row.child_thread_id
            || child_run.turn_id != child_turn_id
            || child_run.parent_run_id != Some(parent_run_id)
            || child_run.root_run_id != Some(expected_root_run_id)
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent delegation child lineage 不一致: {id}"
            )));
        }
        return Ok(row);
    }
    if row.child_turn_id.is_some() || row.child_run_id.is_some() {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 已绑定其他 child run: {id}"
        )));
    }
    if row.status != "pending" {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation 不能在 {} 状态绑定 child run: {id}",
            row.status
        )));
    }
    if is_run_terminal(&parent_run.status) || !parent_run.is_active {
        return Err(AgentDbError::Conflict(format!(
            "Agent parent run 已终止，不能绑定 child run: {parent_run_id}"
        )));
    }
    let child_turn = turn::Entity::find_by_id(child_turn_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child turn 不存在: {child_turn_id}"))
        })?;
    let child_run = run::Entity::find_by_id(child_run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}")))?;
    if child_turn.thread_id != row.child_thread_id
        || child_run.thread_id != row.child_thread_id
        || child_run.turn_id != child_turn_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child run 与 delegation 绑定不一致: {id}"
        )));
    }
    let expected_root_run_id = parent_run.root_run_id.unwrap_or(parent_run.id);
    match (child_run.parent_run_id, child_run.root_run_id) {
        (None, None) => {}
        (Some(parent_id), Some(root_id))
            if parent_id == parent_run_id && root_id == expected_root_run_id => {}
        _ => {
            return Err(AgentDbError::Conflict(format!(
                "Agent child run 已绑定其他 lineage: {child_run_id}"
            )));
        }
    }
    let timestamp = now();
    let mut child_active: run::ActiveModel = child_run.into();
    child_active.parent_run_id = Set(Some(parent_run_id));
    child_active.root_run_id = Set(Some(expected_root_run_id));
    child_active.updated_at = Set(timestamp);
    child_active.update(db).await?;
    let mut active: delegation::ActiveModel = row.into();
    active.child_turn_id = Set(Some(child_turn_id));
    active.child_run_id = Set(Some(child_run_id));
    active.updated_at = Set(timestamp);
    active.update(db).await.map_err(AgentDbError::Database)
}

/// Creates the delegation and child Turn/Run, then binds lineage before the
/// child can be claimed.  The existing primitives remain the single owners of
/// writer admission and lineage validation.
pub async fn create_child_delegation(
    db: &DatabaseConnection,
    input: AgentChildDelegationInput,
) -> Result<(delegation::Model, turn::Model, run::Model, message::Model), AgentDbError> {
    create_child_delegation_inner(
        db,
        input.delegation,
        input.child_turn,
        input.parent_lease_token,
    )
    .await
}

/// Creates a delegation requested directly by the owning user.  Unlike a
/// model-issued delegation there is no parent worker lease to validate; the
/// locked parent Run is still required to be active and in a state that can
/// wait for a child.  A stale parent worker is fenced by clearing its lease in
/// the same transaction.
pub async fn create_user_delegation(
    db: &DatabaseConnection,
    input: AgentUserDelegationInput,
) -> Result<(delegation::Model, turn::Model, run::Model, message::Model), AgentDbError> {
    create_child_delegation_inner(db, input.delegation, input.child_turn, None).await
}

async fn create_child_delegation_inner(
    db: &DatabaseConnection,
    delegation_input: AgentDelegationInput,
    child_turn_input: AgentTurnInput,
    parent_lease_token: Option<String>,
) -> Result<(delegation::Model, turn::Model, run::Model, message::Model), AgentDbError> {
    if delegation_input.child_thread_id != child_turn_input.thread_id {
        return Err(AgentDbError::Conflict(
            "Agent delegation 与 child turn thread 不一致".into(),
        ));
    }
    if parent_lease_token
        .as_deref()
        .is_some_and(|token| token.trim().is_empty())
    {
        return Err(AgentDbError::InvalidArgument(
            "Agent parent run lease token 不能为空".into(),
        ));
    }
    let input = AgentChildDelegationInput {
        delegation: delegation_input,
        child_turn: child_turn_input,
        parent_lease_token,
    };
    let parent_identity = run::Entity::find_by_id(input.delegation.parent_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                input.delegation.parent_run_id
            ))
        })?;
    if parent_identity.thread_id != input.delegation.parent_thread_id {
        return Err(AgentDbError::Conflict(
            "Agent delegation 与 parent run thread 不一致".into(),
        ));
    }
    // Lock every involved Thread before any Run. This matches ordinary turn
    // admission and prevents cancellation/completion from forming a
    // Thread↔Run deadlock across the parent and canonical child threads.
    let tx = db.begin().await?;
    lock_threads_on(
        &tx,
        [
            input.delegation.parent_thread_id,
            input.delegation.child_thread_id,
        ],
    )
    .await?;
    let parent_run = run::Entity::find_by_id(input.delegation.parent_run_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                input.delegation.parent_run_id
            ))
        })?;
    if let Some(existing) = delegation_by_parent_and_key(
        &tx,
        input.delegation.parent_run_id,
        &input.delegation.delegation_key,
    )
    .await?
    {
        validate_delegation_replay(&existing, &input.delegation)?;
        let child_turn_id = existing.child_turn_id.ok_or_else(|| {
            AgentDbError::Conflict(format!(
                "Agent delegation 尚未绑定 child turn: {}",
                existing.id
            ))
        })?;
        let child_run_id = existing.child_run_id.ok_or_else(|| {
            AgentDbError::Conflict(format!(
                "Agent delegation 尚未绑定 child run: {}",
                existing.id
            ))
        })?;
        let child_turn = turn::Entity::find_by_id(child_turn_id)
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!("Agent child turn 不存在: {child_turn_id}"))
            })?;
        let child_run = run::Entity::find_by_id(child_run_id)
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}"))
            })?;
        let child_message = message::Entity::find_by_id(child_turn.input_message_id)
            .one(&tx)
            .await?
            .ok_or_else(|| AgentDbError::NotFound("Agent child turn 缺少输入消息".into()))?;
        tx.commit().await?;
        return Ok((existing, child_turn, child_run, child_message));
    }
    let timestamp = now();
    if let Some(parent_lease_token) = input.parent_lease_token.as_deref() {
        validate_live_run_lease(&parent_run, parent_lease_token, timestamp)?;
    } else {
        if is_run_terminal(&parent_run.status) || !parent_run.is_active {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent run 已终止，不能创建 user delegation: {}",
                parent_run.id
            )));
        }
        if matches!(
            parent_run.status.as_str(),
            "paused" | "pausing" | "cancelling"
        ) || (parent_run.status == "waiting"
            && parent_run.waiting_reason.as_deref() != Some("child"))
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent run 当前状态不能创建 user delegation: {} ({})",
                parent_run.id, parent_run.status
            )));
        }
    }
    let parent_execution = match input.delegation.parent_execution_id {
        Some(execution_id) => {
            let row = execution::Entity::find_by_id(execution_id)
                .lock_exclusive()
                .one(&tx)
                .await?
                .ok_or_else(|| {
                    AgentDbError::NotFound(format!("Agent parent execution 不存在: {execution_id}"))
                })?;
            if row.run_id != parent_run.id
                || row.status != "prepared"
                || row.execution_kind != "delegation"
            {
                return Err(AgentDbError::Conflict(format!(
                    "Agent parent execution 不可用于委派: {execution_id} ({})",
                    row.status
                )));
            }
            Some(row)
        }
        None => None,
    };
    let delegation = create_delegation(&tx, input.delegation.clone()).await?;
    let (child_turn, child_run, child_message) =
        create_turn_with_run_on(&tx, input.child_turn).await?;
    let child_run_id = child_run.id;
    let delegation = bind_delegation_child_on(
        &tx,
        delegation.id,
        delegation.parent_run_id,
        child_turn.id,
        child_run_id,
    )
    .await?;
    // bind_delegation_child_on updates the child lineage; return the refreshed
    // row so callers never observe a pre-bind parent/root run projection.
    let child_run = run::Entity::find_by_id(child_run_id)
        .one(&tx)
        .await?
        .ok_or_else(|| AgentDbError::NotFound(format!("Agent child run 不存在: {child_run_id}")))?;
    let delegation = update_delegation(&tx, delegation.id, "running", None, None, None).await?;
    if let Some(parent_execution) = parent_execution {
        let mut execution_active: execution::ActiveModel = parent_execution.into();
        // A delegation execution represents the whole child lifecycle.  Keep
        // it running until the child returns a terminal result so the parent
        // transcript never exposes a premature tool success.
        execution_active.status = Set("running".to_string());
        execution_active.result_json = Set(Some(json!({
            "delegation_id": delegation.id.to_string(),
            "child_thread_id": delegation.child_thread_id.to_string(),
            "child_turn_id": child_turn.id.to_string(),
            "child_run_id": child_run.id.to_string(),
        })));
        execution_active.result_summary = Set(Some("Child Agent 已启动，等待结果".to_string()));
        execution_active.updated_at = Set(timestamp);
        execution_active.started_at = Set(Some(timestamp));
        execution_active.completed_at = Set(None);
        execution_active.update(&tx).await?;
    }
    let mut parent_active: run::ActiveModel = parent_run.into();
    parent_active.status = Set("waiting".to_string());
    parent_active.waiting_reason = Set(Some("child".to_string()));
    parent_active.worker_id = Set(None);
    parent_active.lease_token = Set(None);
    parent_active.lease_expires_at = Set(None);
    parent_active.updated_at = Set(timestamp);
    parent_active.update(&tx).await?;
    let _ = refresh_thread_status_on(&tx, input.delegation.parent_thread_id).await?;
    let _ = refresh_thread_status_on(&tx, input.delegation.child_thread_id).await?;
    tx.commit().await?;
    Ok((delegation, child_turn, child_run, child_message))
}

/// Completes a child Run, records the delegation result, delivers a durable
/// `child_result` inbox item, and wakes the parent in one transaction.
pub async fn complete_child_delegation(
    db: &DatabaseConnection,
    input: AgentChildCompletionInput,
) -> Result<Option<(run::Model, delegation::Model, inbox::Model, run::Model)>, AgentDbError> {
    let result_summary = input.result_summary.trim().to_string();
    if result_summary.is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent child result_summary 不能为空".into(),
        ));
    }
    let child_identity = run::Entity::find_by_id(input.child_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let delegation_identity = delegation::Entity::find()
        .filter(delegation::Column::ChildRunId.eq(input.child_run_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent child run 缺少 delegation: {}",
                input.child_run_id
            ))
        })?;
    let parent_identity = run::Entity::find_by_id(delegation_identity.parent_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                delegation_identity.parent_run_id
            ))
        })?;
    if child_identity.thread_id != delegation_identity.child_thread_id
        || parent_identity.thread_id != delegation_identity.parent_thread_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation lineage 不一致: {}",
            delegation_identity.id
        )));
    }
    let tx = db.begin().await?;
    lock_threads_on(&tx, [child_identity.thread_id, parent_identity.thread_id]).await?;
    let locked_runs = lock_runs_on(&tx, [child_identity.id, parent_identity.id]).await?;
    let child_run = locked_runs
        .iter()
        .find(|row| row.id == child_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let parent_run = locked_runs
        .iter()
        .find(|row| row.id == parent_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent parent run 不存在: {}", parent_identity.id))
        })?;
    let timestamp = now();
    validate_live_run_lease(&child_run, &input.child_lease_token, timestamp)?;
    if requeue_for_new_input_on(&tx, &child_run, input.observed_input_message_id).await? {
        tx.commit().await?;
        return Ok(None);
    }
    let child_turn = turn::Entity::find_by_id(child_run.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child turn 不存在: {}", child_run.turn_id))
        })?;
    let mut delegation = lock_delegations_on(&tx, [delegation_identity.id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent delegation 不存在: {}",
                delegation_identity.id
            ))
        })?;
    let parent_execution = if let Some(execution_id) = delegation.parent_execution_id {
        let row = execution::Entity::find_by_id(execution_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!(
                    "Agent parent delegation execution 不存在: {execution_id}"
                ))
            })?;
        if row.run_id != parent_run.id
            || row.execution_kind != "delegation"
            || row.tool_name.as_deref() != Some("delegate")
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent delegation execution 关联不一致: {execution_id}"
            )));
        }
        Some(row)
    } else {
        None
    };
    if delegation.child_run_id != Some(child_run.id)
        || delegation.parent_run_id != parent_run.id
        || delegation.status != "running"
        || child_turn.thread_id != child_run.thread_id
        || input.final_message.thread_id != child_run.thread_id
        || input.final_message.turn_id != Some(child_run.turn_id)
        || input.final_message.run_id != Some(child_run.id)
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child completion 关联记录不一致: {}",
            input.child_run_id
        )));
    }
    let final_message = append_message_on(&tx, input.final_message).await?;
    complete_turn(
        &tx,
        child_run.turn_id,
        "completed",
        Some(final_message.id),
        input.result_refs_json.clone(),
        None,
    )
    .await?;
    let mut child_active: run::ActiveModel = child_run.into();
    child_active.status = Set("completed".to_string());
    child_active.waiting_reason = Set(None);
    child_active.is_active = Set(false);
    child_active.worker_id = Set(None);
    child_active.lease_token = Set(None);
    child_active.lease_expires_at = Set(None);
    child_active.updated_at = Set(timestamp);
    child_active.completed_at = Set(Some(timestamp));
    let child_run = child_active.update(&tx).await?;

    let mut delegation_active: delegation::ActiveModel = delegation.clone().into();
    delegation_active.status = Set("awaiting_acceptance".to_string());
    delegation_active.result_refs_json = Set(input.result_refs_json.clone());
    delegation_active.result_summary = Set(Some(result_summary.clone()));
    delegation_active.updated_at = Set(timestamp);
    delegation = delegation_active.update(&tx).await?;

    let parent_tool_message = if let Some(parent_execution) = parent_execution.as_ref() {
        let result = json!({
            "delegation_id": delegation.id.to_string(),
            "child_run_id": child_run.id.to_string(),
            "result_refs": input.result_refs_json.clone(),
            "summary": result_summary.clone(),
        });
        let message = append_parent_delegation_tool_message_on(
            &tx,
            &parent_run,
            &child_run,
            &delegation,
            parent_execution,
            "succeeded",
            result.clone(),
        )
        .await?;
        finish_parent_delegation_execution_on(
            &tx,
            parent_execution.clone(),
            "succeeded",
            result,
            result_summary.clone(),
            input.result_refs_json.clone(),
            None,
            timestamp,
        )
        .await?;
        Some(message)
    } else {
        None
    };

    let mut inbox_payload = json!({
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "result_refs": input.result_refs_json,
    });
    if let Some(message) = parent_tool_message.as_ref() {
        inbox_payload["message_id"] = json!(message.id.to_string());
    }
    let inbox_row = enqueue_inbox(
        &tx,
        AgentInboxInput {
            thread_id: delegation.parent_thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            source_kind: "subagent".to_string(),
            source_thread_id: Some(delegation.child_thread_id),
            inbox_kind: "child_result".to_string(),
            delivery_mode: "followup".to_string(),
            priority: 20,
            dedupe_key: format!("child-result:{}", delegation.id),
            content: result_summary,
            payload_json: Some(inbox_payload),
            available_at: None,
        },
    )
    .await?;
    let parent_run = if parent_run.is_active
        && parent_run.status == "waiting"
        && parent_run.waiting_reason.as_deref() == Some("child")
    {
        let mut parent_active: run::ActiveModel = parent_run.into();
        parent_active.status = Set("queued".to_string());
        parent_active.waiting_reason = Set(None);
        parent_active.worker_id = Set(None);
        parent_active.lease_token = Set(None);
        parent_active.lease_expires_at = Set(None);
        parent_active.next_attempt_at = Set(timestamp);
        parent_active.updated_at = Set(timestamp);
        parent_active.update(&tx).await?
    } else {
        parent_run
    };
    append_run_event_on(
        &tx,
        &child_run,
        "run_completed",
        "completed",
        "Child Agent 已完成",
        None,
        Some(delegation.id),
        Some(final_message.id),
        Some(json!({ "result_summary": delegation.result_summary.clone() })),
    )
    .await?;
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            execution_id: delegation.parent_execution_id,
            delegation_id: Some(delegation.id),
            message_id: parent_tool_message.as_ref().map(|message| message.id),
            actor_kind: "agent".into(),
            actor_id: Some(child_run.id.to_string()),
            kind: "child_completed".into(),
            status: "awaiting_acceptance".into(),
            public_text: "Child Agent 已完成，等待主 Agent 验收".into(),
            payload_json: Some(json!({
                "delegation_id": delegation.id.to_string(),
                "child_run_id": child_run.id.to_string(),
                "result_summary": delegation.result_summary.clone(),
            })),
        },
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, child_run.thread_id).await?;
    let _ = refresh_thread_status_on(&tx, parent_run.thread_id).await?;
    tx.commit().await?;
    Ok(Some((child_run, delegation, inbox_row, parent_run)))
}

/// Fails a child Run and its delegation, then delivers the failure to and
/// wakes the parent atomically. Repeating the same failure returns the durable
/// terminal records instead of creating another inbox item.
pub async fn fail_child_delegation(
    db: &DatabaseConnection,
    input: AgentChildFailureInput,
) -> Result<(run::Model, delegation::Model, inbox::Model, run::Model), AgentDbError> {
    let error_message = input.error_message.trim().to_string();
    if error_message.is_empty() {
        return Err(AgentDbError::InvalidArgument(
            "Agent child error_message 不能为空".into(),
        ));
    }
    let child_identity = run::Entity::find_by_id(input.child_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let delegation_identity = delegation::Entity::find()
        .filter(delegation::Column::ChildRunId.eq(input.child_run_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent child run 缺少 delegation: {}",
                input.child_run_id
            ))
        })?;
    let parent_identity = run::Entity::find_by_id(delegation_identity.parent_run_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent parent run 不存在: {}",
                delegation_identity.parent_run_id
            ))
        })?;
    if child_identity.thread_id != delegation_identity.child_thread_id
        || parent_identity.thread_id != delegation_identity.parent_thread_id
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent delegation lineage 不一致: {}",
            delegation_identity.id
        )));
    }

    let tx = db.begin().await?;
    lock_threads_on(&tx, [child_identity.thread_id, parent_identity.thread_id]).await?;
    let locked_runs = lock_runs_on(&tx, [child_identity.id, parent_identity.id]).await?;
    let child_run = locked_runs
        .iter()
        .find(|row| row.id == child_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child run 不存在: {}", input.child_run_id))
        })?;
    let parent_run = locked_runs
        .iter()
        .find(|row| row.id == parent_identity.id)
        .cloned()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent parent run 不存在: {}", parent_identity.id))
        })?;
    let timestamp = now();
    let mut delegation = lock_delegations_on(&tx, [delegation_identity.id])
        .await?
        .into_iter()
        .next()
        .ok_or_else(|| {
            AgentDbError::NotFound(format!(
                "Agent delegation 不存在: {}",
                delegation_identity.id
            ))
        })?;
    let parent_execution = if let Some(execution_id) = delegation.parent_execution_id {
        let row = execution::Entity::find_by_id(execution_id)
            .lock_exclusive()
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::NotFound(format!(
                    "Agent parent delegation execution 不存在: {execution_id}"
                ))
            })?;
        if row.run_id != parent_run.id
            || row.execution_kind != "delegation"
            || row.tool_name.as_deref() != Some("delegate")
        {
            return Err(AgentDbError::Conflict(format!(
                "Agent parent delegation execution 关联不一致: {execution_id}"
            )));
        }
        Some(row)
    } else {
        None
    };

    if child_run.status == "failed" && delegation.status == "failed" {
        let inbox_row = inbox::Entity::find()
            .filter(inbox::Column::ThreadId.eq(parent_run.thread_id))
            .filter(inbox::Column::DedupeKey.eq(format!("child-failed:{}", delegation.id)))
            .one(&tx)
            .await?
            .ok_or_else(|| {
                AgentDbError::Conflict(format!("Agent child failure inbox 缺失: {}", delegation.id))
            })?;
        tx.commit().await?;
        return Ok((child_run, delegation, inbox_row, parent_run));
    }
    validate_live_run_lease(&child_run, &input.child_lease_token, timestamp)?;
    if delegation.child_run_id != Some(child_run.id)
        || delegation.parent_run_id != parent_run.id
        || delegation.status != "running"
    {
        return Err(AgentDbError::Conflict(format!(
            "Agent child failure 关联记录不一致: {}",
            input.child_run_id
        )));
    }
    let child_turn = turn::Entity::find_by_id(child_run.turn_id)
        .lock_exclusive()
        .one(&tx)
        .await?
        .ok_or_else(|| {
            AgentDbError::NotFound(format!("Agent child turn 不存在: {}", child_run.turn_id))
        })?;
    if child_turn.thread_id != child_run.thread_id {
        return Err(AgentDbError::Conflict(format!(
            "Agent child turn 与 run 不一致: {}",
            child_run.id
        )));
    }

    let mut turn_active: turn::ActiveModel = child_turn.into();
    turn_active.status = Set("failed".to_string());
    turn_active.error_message = Set(Some(error_message.clone()));
    turn_active.updated_at = Set(timestamp);
    turn_active.completed_at = Set(Some(timestamp));
    turn_active.update(&tx).await?;

    let mut child_active: run::ActiveModel = child_run.into();
    child_active.status = Set("failed".to_string());
    child_active.waiting_reason = Set(None);
    child_active.is_active = Set(false);
    child_active.last_error = Set(Some(error_message.clone()));
    child_active.worker_id = Set(None);
    child_active.lease_token = Set(None);
    child_active.lease_expires_at = Set(None);
    child_active.updated_at = Set(timestamp);
    child_active.completed_at = Set(Some(timestamp));
    let child_run = child_active.update(&tx).await?;
    terminate_executions_on(
        &tx,
        [child_run.id],
        ExecutionTermination::Failed,
        Some(error_message.as_str()),
        timestamp,
    )
    .await?;
    interrupt_streaming_messages_on(&tx, [child_run.id], Some(error_message.as_str())).await?;

    let mut delegation_active: delegation::ActiveModel = delegation.into();
    delegation_active.status = Set("failed".to_string());
    delegation_active.error_message = Set(Some(error_message.clone()));
    delegation_active.updated_at = Set(timestamp);
    delegation_active.completed_at = Set(Some(timestamp));
    delegation = delegation_active.update(&tx).await?;

    let parent_tool_message = if let Some(parent_execution) = parent_execution.as_ref() {
        let result = json!({
            "delegation_id": delegation.id.to_string(),
            "child_run_id": child_run.id.to_string(),
            "error": error_message.clone(),
        });
        let message = append_parent_delegation_tool_message_on(
            &tx,
            &parent_run,
            &child_run,
            &delegation,
            parent_execution,
            "failed",
            result.clone(),
        )
        .await?;
        finish_parent_delegation_execution_on(
            &tx,
            parent_execution.clone(),
            "failed",
            result,
            "Child Agent 执行失败".into(),
            None,
            Some(error_message.clone()),
            timestamp,
        )
        .await?;
        Some(message)
    } else {
        None
    };

    let mut inbox_payload = json!({
        "delegation_id": delegation.id.to_string(),
        "child_run_id": child_run.id.to_string(),
        "error": error_message,
    });
    if let Some(message) = parent_tool_message.as_ref() {
        inbox_payload["message_id"] = json!(message.id.to_string());
    }
    let inbox_row = enqueue_inbox(
        &tx,
        AgentInboxInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            source_kind: "subagent".into(),
            source_thread_id: Some(child_run.thread_id),
            inbox_kind: "child_result".into(),
            delivery_mode: "followup".into(),
            priority: 20,
            dedupe_key: format!("child-failed:{}", delegation.id),
            content: format!(
                "Child Agent failed: {}",
                inbox_payload["error"].as_str().unwrap_or("unknown error")
            ),
            payload_json: Some(inbox_payload),
            available_at: None,
        },
    )
    .await?;
    let parent_run = if parent_run.is_active
        && parent_run.status == "waiting"
        && parent_run.waiting_reason.as_deref() == Some("child")
    {
        let mut parent_active: run::ActiveModel = parent_run.into();
        parent_active.status = Set("queued".to_string());
        parent_active.waiting_reason = Set(None);
        parent_active.worker_id = Set(None);
        parent_active.lease_token = Set(None);
        parent_active.lease_expires_at = Set(None);
        parent_active.next_attempt_at = Set(timestamp);
        parent_active.updated_at = Set(timestamp);
        parent_active.update(&tx).await?
    } else {
        parent_run
    };
    append_run_event_on(
        &tx,
        &child_run,
        "run_failed",
        "failed",
        "Child Agent 执行失败",
        None,
        Some(delegation.id),
        None,
        Some(json!({ "error": delegation.error_message.clone() })),
    )
    .await?;
    append_event_on(
        &tx,
        AgentEventInput {
            thread_id: parent_run.thread_id,
            turn_id: Some(parent_run.turn_id),
            run_id: Some(parent_run.id),
            execution_id: delegation.parent_execution_id,
            delegation_id: Some(delegation.id),
            message_id: parent_tool_message.as_ref().map(|message| message.id),
            actor_kind: "agent".into(),
            actor_id: Some(child_run.id.to_string()),
            kind: "child_failed".into(),
            status: "failed".into(),
            public_text: "Child Agent 执行失败，已通知主 Agent".into(),
            payload_json: Some(json!({
                "delegation_id": delegation.id.to_string(),
                "child_run_id": child_run.id.to_string(),
                "error": delegation.error_message.clone(),
            })),
        },
    )
    .await?;
    let _ = refresh_thread_status_on(&tx, child_run.thread_id).await?;
    let _ = refresh_thread_status_on(&tx, parent_run.thread_id).await?;
    tx.commit().await?;
    Ok((child_run, delegation, inbox_row, parent_run))
}

pub async fn insert_memory<C>(
    db: &C,
    input: AgentMemoryInput,
) -> Result<memory::Model, AgentDbError>
where
    C: ConnectionTrait,
{
    if input.revision <= 0 {
        return Err(AgentDbError::InvalidArgument(
            "memory revision 必须为正数".into(),
        ));
    }
    let timestamp = now();
    memory::ActiveModel {
        owner_user_id: Set(input.owner_user_id),
        project_id: Set(input.project_id),
        thread_id: Set(input.thread_id),
        scope_kind: Set(input.scope_kind),
        scope_key: Set(input.scope_key),
        memory_key: Set(input.memory_key),
        revision: Set(input.revision),
        status: Set(input.status),
        content: Set(input.content),
        content_json: Set(input.content_json),
        source_message_id: Set(input.source_message_id),
        source_execution_id: Set(input.source_execution_id),
        supersedes_memory_id: Set(input.supersedes_memory_id),
        created_at: Set(timestamp),
        updated_at: Set(timestamp),
        retracted_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AgentDbError::Database)
}

pub async fn list_memory<C>(
    db: &C,
    owner_user_id: i64,
    scope_kind: Option<&str>,
    scope_key: Option<&str>,
) -> Result<Vec<memory::Model>, AgentDbError>
where
    C: ConnectionTrait,
{
    let mut query = memory::Entity::find()
        .filter(memory::Column::OwnerUserId.eq(owner_user_id))
        .order_by_desc(memory::Column::CreatedAt);
    if let Some(kind) = scope_kind {
        query = query.filter(memory::Column::ScopeKind.eq(kind));
    }
    if let Some(key) = scope_key {
        query = query.filter(memory::Column::ScopeKey.eq(key));
    }
    query.all(db).await.map_err(AgentDbError::Database)
}
