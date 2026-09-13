//! Database-free contracts shared by the Agent control plane and worker.
//!
//! SeaORM rows stay in `a4n-agent-application`; this crate contains only the
//! stable snapshots, worker inputs and ports needed across the process
//! boundary.

use a4n_api_contract::agent::{AgentBudget, AgentResourceRef, AgentScope};
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_worker_contract::{
    AgentWorkerApprovalCheckRequest, AgentWorkerApprovalCheckSnapshotWire,
    AgentWorkerApprovalRequest, AgentWorkerBudgetRequest, AgentWorkerBudgetSnapshotWire,
    AgentWorkerDelegationSnapshotWire, AgentWorkerExecutionCreateRequest,
    AgentWorkerExecutionLookupRequest, AgentWorkerExecutionSnapshotWire,
    AgentWorkerExecutionUpdateRequest, AgentWorkerFailureRequest, AgentWorkerInboxSnapshotWire,
    AgentWorkerMessageRequest, AgentWorkerMessageSnapshotWire, AgentWorkerRunContextWire,
    AgentWorkerRunSnapshotWire, AgentWorkerRunWire, AgentWorkerThreadSnapshotWire,
    AgentWorkerTurnSnapshotWire,
};
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const MAX_CONTEXT_SUMMARY_BYTES: usize = 256 * 1024;

pub fn model_request_snapshot(request: &ChatRequest) -> Result<Value, AppError> {
    request.snapshot_value()
}

pub fn restore_model_request_snapshot(mut value: Value) -> Result<ChatRequest, AppError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| AppError::InvalidArgument("模型请求快照必须是对象".into()))?;
    let trace = object.remove("trace_context");
    let manifest = object
        .remove("context_manifest_json")
        .filter(|value| !value.is_null());
    let mut request: ChatRequest = serde_json::from_value(value)?;
    request.trace_context = trace.map(serde_json::from_value).transpose()?.flatten();
    request.context_manifest_json = manifest;
    Ok(request)
}

/// Stable worker-facing projection of a durable Agent run.
#[derive(Debug, Clone)]
pub struct AgentRunSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: i64,
    pub parent_run_id: Option<i64>,
    pub attempt: i32,
    pub status: String,
    pub budget_json: Value,
    pub usage_json: Value,
    pub worker_id: Option<String>,
    pub lease_token: Option<String>,
    pub lease_expires_at: Option<NaiveDateTime>,
    pub created_at: NaiveDateTime,
    pub started_at: Option<NaiveDateTime>,
}

/// Durable facts carried between Agent runs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentContextSummary {
    pub current_goal: String,
    pub constraints: Vec<String>,
    pub decisions: Vec<String>,
    pub facts_resources: Vec<String>,
    pub completed: Vec<String>,
    pub pending: Vec<String>,
    pub tool_state: Vec<String>,
    pub covered_through_sequence: i64,
}

impl AgentContextSummary {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.covered_through_sequence < 0 || self.current_goal.chars().count() > 4_000 {
            return Err(AppError::InvalidState {
                current: "Agent context_summary 的目标或覆盖序列无效".into(),
                operation: "校验 Agent context_summary".into(),
            });
        }
        for (name, values) in [
            ("constraints", &self.constraints),
            ("decisions", &self.decisions),
            ("facts_resources", &self.facts_resources),
            ("completed", &self.completed),
            ("pending", &self.pending),
            ("tool_state", &self.tool_state),
        ] {
            if values.len() > 64 || values.iter().any(|value| value.chars().count() > 4_000) {
                return Err(AppError::InvalidState {
                    current: format!("Agent context_summary.{name} 超出大小限制"),
                    operation: "校验 Agent context_summary".into(),
                });
            }
        }
        if serde_json::to_vec(self)?.len() > MAX_CONTEXT_SUMMARY_BYTES {
            return Err(AppError::InvalidState {
                current: format!(
                    "Agent context_summary 超过 {} KiB 限制",
                    MAX_CONTEXT_SUMMARY_BYTES / 1024
                ),
                operation: "校验 Agent context_summary".into(),
            });
        }
        Ok(())
    }
}

pub fn parse_agent_context_summary(raw: Option<&str>) -> Result<Option<Value>, AppError> {
    let Some(raw) = raw else {
        return Ok(None);
    };
    let summary: AgentContextSummary =
        serde_json::from_str(raw).map_err(|error| AppError::InvalidState {
            current: format!("Agent thread context_summary 无效: {error}"),
            operation: "读取 Agent context_summary".into(),
        })?;
    summary.validate()?;
    serde_json::to_value(summary)
        .map(Some)
        .map_err(AppError::from)
}

#[derive(Debug, Clone)]
pub struct AgentThreadSnapshot {
    pub id: i64,
    pub owner_user_id: i64,
    pub thread_kind: String,
    pub profile_id: String,
    pub title: Option<String>,
    pub project_id: Option<i64>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub scope_json: Value,
    pub default_model_config_id: Option<i64>,
    pub status: String,
    pub revision: i64,
    pub context_summary: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AgentTurnSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub input_message_id: i64,
    pub execution_mode: String,
    pub goal: Option<String>,
    pub model_override_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AgentMessageSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub role: String,
    pub content: String,
    pub payload_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct AgentExecutionSnapshot {
    pub id: i64,
    pub run_id: i64,
    pub execution_key: String,
    pub execution_kind: String,
    pub status: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<Value>,
    pub resource_preconditions_json: Option<Value>,
    pub result_json: Option<Value>,
    pub result_summary: Option<String>,
    pub result_refs_json: Option<Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentDelegationSnapshot {
    pub id: i64,
    pub parent_run_id: i64,
    pub parent_execution_id: Option<i64>,
    pub child_thread_id: i64,
    pub child_turn_id: Option<i64>,
    pub child_run_id: Option<i64>,
    pub delegation_key: String,
    pub capability: String,
    pub objective: String,
    pub success_criteria_json: Value,
    pub scope_json: Value,
    pub budget_json: Value,
    pub status: String,
    pub result_refs_json: Option<Value>,
    pub result_summary: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AgentInboxSnapshot {
    pub id: i64,
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub source_kind: String,
    pub source_thread_id: Option<i64>,
    pub inbox_kind: String,
    pub delivery_mode: String,
    pub content: String,
    pub payload_json: Option<Value>,
    pub status: String,
    pub claim_token: Option<String>,
}

impl From<AgentWorkerRunSnapshotWire> for AgentRunSnapshot {
    fn from(snapshot: AgentWorkerRunSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            turn_id: snapshot.turn_id,
            parent_run_id: snapshot.parent_run_id,
            attempt: snapshot.attempt,
            status: snapshot.status,
            budget_json: snapshot.budget_json,
            usage_json: snapshot.usage_json,
            worker_id: snapshot.worker_id,
            lease_token: snapshot.lease_token,
            lease_expires_at: snapshot.lease_expires_at,
            created_at: snapshot.created_at,
            started_at: snapshot.started_at,
        }
    }
}

impl From<AgentRunSnapshot> for AgentWorkerRunSnapshotWire {
    fn from(snapshot: AgentRunSnapshot) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            turn_id: snapshot.turn_id,
            parent_run_id: snapshot.parent_run_id,
            attempt: snapshot.attempt,
            status: snapshot.status,
            budget_json: snapshot.budget_json,
            usage_json: snapshot.usage_json,
            worker_id: snapshot.worker_id,
            lease_token: snapshot.lease_token,
            lease_expires_at: snapshot.lease_expires_at,
            created_at: snapshot.created_at,
            started_at: snapshot.started_at,
        }
    }
}

impl From<AgentWorkerThreadSnapshotWire> for AgentThreadSnapshot {
    fn from(snapshot: AgentWorkerThreadSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            owner_user_id: snapshot.owner_user_id,
            thread_kind: snapshot.thread_kind,
            profile_id: snapshot.profile_id,
            title: snapshot.title,
            project_id: snapshot.project_id,
            resource_type: snapshot.resource_type,
            resource_id: snapshot.resource_id,
            scope_json: snapshot.scope_json,
            default_model_config_id: snapshot.default_model_config_id,
            status: snapshot.status,
            revision: snapshot.revision,
            context_summary: snapshot.context_summary,
        }
    }
}

impl From<AgentThreadSnapshot> for AgentWorkerThreadSnapshotWire {
    fn from(snapshot: AgentThreadSnapshot) -> Self {
        Self {
            id: snapshot.id,
            owner_user_id: snapshot.owner_user_id,
            thread_kind: snapshot.thread_kind,
            profile_id: snapshot.profile_id,
            title: snapshot.title,
            project_id: snapshot.project_id,
            resource_type: snapshot.resource_type,
            resource_id: snapshot.resource_id,
            scope_json: snapshot.scope_json,
            default_model_config_id: snapshot.default_model_config_id,
            status: snapshot.status,
            revision: snapshot.revision,
            context_summary: snapshot.context_summary,
        }
    }
}

impl From<AgentWorkerTurnSnapshotWire> for AgentTurnSnapshot {
    fn from(snapshot: AgentWorkerTurnSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            input_message_id: snapshot.input_message_id,
            execution_mode: snapshot.execution_mode,
            goal: snapshot.goal,
            model_override_json: snapshot.model_override_json,
        }
    }
}

impl From<AgentTurnSnapshot> for AgentWorkerTurnSnapshotWire {
    fn from(snapshot: AgentTurnSnapshot) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            input_message_id: snapshot.input_message_id,
            execution_mode: snapshot.execution_mode,
            goal: snapshot.goal,
            model_override_json: snapshot.model_override_json,
        }
    }
}

impl From<AgentWorkerMessageSnapshotWire> for AgentMessageSnapshot {
    fn from(snapshot: AgentWorkerMessageSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            turn_id: snapshot.turn_id,
            run_id: snapshot.run_id,
            role: snapshot.role,
            content: snapshot.content,
            payload_json: snapshot.payload_json,
        }
    }
}

impl From<AgentMessageSnapshot> for AgentWorkerMessageSnapshotWire {
    fn from(snapshot: AgentMessageSnapshot) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            turn_id: snapshot.turn_id,
            run_id: snapshot.run_id,
            role: snapshot.role,
            content: snapshot.content,
            payload_json: snapshot.payload_json,
        }
    }
}

impl From<AgentWorkerExecutionSnapshotWire> for AgentExecutionSnapshot {
    fn from(snapshot: AgentWorkerExecutionSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            run_id: snapshot.run_id,
            execution_key: snapshot.execution_key,
            execution_kind: snapshot.execution_kind,
            status: snapshot.status,
            tool_name: snapshot.tool_name,
            effect: snapshot.effect,
            replay_policy: snapshot.replay_policy,
            arguments_json: snapshot.arguments_json,
            resource_preconditions_json: snapshot.resource_preconditions_json,
            result_json: snapshot.result_json,
            result_summary: snapshot.result_summary,
            result_refs_json: snapshot.result_refs_json,
            error_message: snapshot.error_message,
        }
    }
}

impl From<AgentExecutionSnapshot> for AgentWorkerExecutionSnapshotWire {
    fn from(snapshot: AgentExecutionSnapshot) -> Self {
        Self {
            id: snapshot.id,
            run_id: snapshot.run_id,
            execution_key: snapshot.execution_key,
            execution_kind: snapshot.execution_kind,
            status: snapshot.status,
            tool_name: snapshot.tool_name,
            effect: snapshot.effect,
            replay_policy: snapshot.replay_policy,
            arguments_json: snapshot.arguments_json,
            resource_preconditions_json: snapshot.resource_preconditions_json,
            result_json: snapshot.result_json,
            result_summary: snapshot.result_summary,
            result_refs_json: snapshot.result_refs_json,
            error_message: snapshot.error_message,
        }
    }
}

impl From<AgentWorkerDelegationSnapshotWire> for AgentDelegationSnapshot {
    fn from(snapshot: AgentWorkerDelegationSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            parent_run_id: snapshot.parent_run_id,
            parent_execution_id: snapshot.parent_execution_id,
            child_thread_id: snapshot.child_thread_id,
            child_turn_id: snapshot.child_turn_id,
            child_run_id: snapshot.child_run_id,
            delegation_key: snapshot.delegation_key,
            capability: snapshot.capability,
            objective: snapshot.objective,
            success_criteria_json: snapshot.success_criteria_json,
            scope_json: snapshot.scope_json,
            budget_json: snapshot.budget_json,
            status: snapshot.status,
            result_refs_json: snapshot.result_refs_json,
            result_summary: snapshot.result_summary,
            error_message: snapshot.error_message,
        }
    }
}

impl From<AgentDelegationSnapshot> for a4n_worker_contract::AgentWorkerDelegationSnapshotWire {
    fn from(snapshot: AgentDelegationSnapshot) -> Self {
        Self {
            id: snapshot.id,
            parent_run_id: snapshot.parent_run_id,
            parent_execution_id: snapshot.parent_execution_id,
            child_thread_id: snapshot.child_thread_id,
            child_turn_id: snapshot.child_turn_id,
            child_run_id: snapshot.child_run_id,
            delegation_key: snapshot.delegation_key,
            capability: snapshot.capability,
            objective: snapshot.objective,
            success_criteria_json: snapshot.success_criteria_json,
            scope_json: snapshot.scope_json,
            budget_json: snapshot.budget_json,
            status: snapshot.status,
            result_refs_json: snapshot.result_refs_json,
            result_summary: snapshot.result_summary,
            error_message: snapshot.error_message,
        }
    }
}

impl From<AgentWorkerInboxSnapshotWire> for AgentInboxSnapshot {
    fn from(snapshot: AgentWorkerInboxSnapshotWire) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            turn_id: snapshot.turn_id,
            run_id: snapshot.run_id,
            source_kind: snapshot.source_kind,
            source_thread_id: snapshot.source_thread_id,
            inbox_kind: snapshot.inbox_kind,
            delivery_mode: snapshot.delivery_mode,
            content: snapshot.content,
            payload_json: snapshot.payload_json,
            status: snapshot.status,
            claim_token: snapshot.claim_token,
        }
    }
}

impl From<AgentInboxSnapshot> for AgentWorkerInboxSnapshotWire {
    fn from(snapshot: AgentInboxSnapshot) -> Self {
        Self {
            id: snapshot.id,
            thread_id: snapshot.thread_id,
            turn_id: snapshot.turn_id,
            run_id: snapshot.run_id,
            source_kind: snapshot.source_kind,
            source_thread_id: snapshot.source_thread_id,
            inbox_kind: snapshot.inbox_kind,
            delivery_mode: snapshot.delivery_mode,
            content: snapshot.content,
            payload_json: snapshot.payload_json,
            status: snapshot.status,
            claim_token: snapshot.claim_token,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AgentWorkerRun {
    pub run: AgentRunSnapshot,
    pub thread: AgentThreadSnapshot,
    pub lease_token: String,
}

impl AgentWorkerRun {
    pub fn to_wire(&self) -> AgentWorkerRunWire {
        AgentWorkerRunWire {
            run: self.run.clone().into(),
            thread: self.thread.clone().into(),
            lease_token: self.lease_token.clone(),
        }
    }

    pub fn from_wire(wire: AgentWorkerRunWire) -> Result<Self, AppError> {
        if wire.lease_token.trim().is_empty()
            || wire.run.lease_token.as_deref() != Some(wire.lease_token.as_str())
        {
            return Err(AppError::InvalidState {
                current: "Agent worker run wire 的 lease token 无效".into(),
                operation: "恢复 Agent worker run 快照".into(),
            });
        }
        Ok(Self {
            run: wire.run.into(),
            thread: wire.thread.into(),
            lease_token: wire.lease_token,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AgentRunContext {
    pub run: AgentRunSnapshot,
    pub thread: AgentThreadSnapshot,
    pub turn: AgentTurnSnapshot,
    pub messages: Vec<AgentMessageSnapshot>,
    pub delegations: Vec<AgentDelegationSnapshot>,
    pub executions: Vec<AgentExecutionSnapshot>,
    pub editor_source: Option<AgentEditorSource>,
    pub owner_user_id: i64,
    pub scope: AgentScope,
    pub profile_id: String,
    pub default_model_config_id: Option<i64>,
    pub execution_mode: String,
    pub goal: Option<String>,
    pub budget: Value,
    pub usage: Value,
    pub runtime_snapshot_required: bool,
    pub delegation_depth: u8,
}

impl AgentRunContext {
    pub fn to_wire(&self) -> AgentWorkerRunContextWire {
        AgentWorkerRunContextWire {
            run: self.run.clone().into(),
            thread: self.thread.clone().into(),
            turn: self.turn.clone().into(),
            messages: self.messages.iter().cloned().map(Into::into).collect(),
            delegations: self.delegations.iter().cloned().map(Into::into).collect(),
            executions: self.executions.iter().cloned().map(Into::into).collect(),
            editor_source: self.editor_source.clone(),
            owner_user_id: self.owner_user_id,
            scope: self.scope.clone(),
            profile_id: self.profile_id.clone(),
            default_model_config_id: self.default_model_config_id,
            execution_mode: self.execution_mode.clone(),
            goal: self.goal.clone(),
            budget: self.budget.clone(),
            usage: self.usage.clone(),
            runtime_snapshot_required: self.runtime_snapshot_required,
            delegation_depth: self.delegation_depth,
        }
    }

    pub fn from_wire(wire: AgentWorkerRunContextWire) -> Self {
        Self {
            run: wire.run.into(),
            thread: wire.thread.into(),
            turn: wire.turn.into(),
            messages: wire.messages.into_iter().map(Into::into).collect(),
            delegations: wire.delegations.into_iter().map(Into::into).collect(),
            executions: wire.executions.into_iter().map(Into::into).collect(),
            editor_source: wire.editor_source,
            owner_user_id: wire.owner_user_id,
            scope: wire.scope,
            profile_id: wire.profile_id,
            default_model_config_id: wire.default_model_config_id,
            execution_mode: wire.execution_mode,
            goal: wire.goal,
            budget: wire.budget,
            usage: wire.usage,
            runtime_snapshot_required: wire.runtime_snapshot_required,
            delegation_depth: wire.delegation_depth,
        }
    }
}

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

#[derive(Debug, Clone)]
pub struct WorkerExecutionInput {
    pub run_id: i64,
    pub lease_token: String,
    pub execution_key: String,
    pub execution_kind: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<Value>,
    pub resource_preconditions_json: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct WorkerExecutionUpdate {
    pub run_id: i64,
    pub lease_token: String,
    pub execution_id: i64,
    pub status: String,
    pub result_json: Option<Value>,
    pub result_summary: Option<String>,
    pub result_refs_json: Option<Value>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone)]
pub struct WorkerUsage {
    pub run_id: i64,
    pub lease_token: String,
    pub usage_json: Value,
}

#[derive(Debug, Clone)]
pub struct WorkerDelegationInput {
    pub capability: String,
    pub objective: String,
    pub delegation_key: String,
    pub child_thread_id: i64,
    pub scope: AgentScope,
    pub success_criteria: Vec<String>,
    pub budget_json: Value,
    pub parent_execution_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct WorkerDelegationResult {
    pub delegation: AgentDelegationSnapshot,
    pub child_turn: AgentTurnSnapshot,
    pub child_run: AgentRunSnapshot,
    pub child_message: AgentMessageSnapshot,
}

#[derive(Debug, Clone)]
pub struct EnsureCanonicalSubagentThreadInput {
    pub canonical_binding_key: String,
    pub capability: String,
    pub title: Option<String>,
    pub scope: AgentScope,
    pub bound_resource: Option<AgentResourceRef>,
    pub default_model_config_id: Option<i64>,
}

pub fn validate_agent_budget(budget: &AgentBudget) -> Result<(), AppError> {
    let positive = [
        budget.max_duration_seconds,
        budget.max_tokens,
        budget.max_cost_micros,
    ]
    .into_iter()
    .flatten()
    .all(|value| value > 0)
        && [
            budget.max_steps,
            budget.max_model_calls,
            budget.max_tool_calls,
        ]
        .into_iter()
        .flatten()
        .all(|value| value > 0)
        && [budget.max_parallel_tools, budget.max_children]
            .into_iter()
            .flatten()
            .all(|value| value > 0)
        && budget.max_delegation_depth.is_none_or(|value| value > 0);
    if !positive {
        return Err(AppError::InvalidArgument(
            "Agent budget 上限必须为正数".into(),
        ));
    }
    if budget
        .max_duration_seconds
        .is_some_and(|value| value > i64::MAX as u64)
    {
        return Err(AppError::InvalidArgument(
            "Agent max_duration_seconds 超出时间实现上限".into(),
        ));
    }
    if budget.max_cost_micros.is_some() {
        return Err(AppError::InvalidArgument(
            "当前 provider 没有可靠的 Agent cost usage，暂不支持 max_cost_micros".into(),
        ));
    }
    Ok(())
}

pub fn enforce_delegation_limits(
    delegations: &[AgentDelegationSnapshot],
    run_id: i64,
    depth: u8,
    budget: &AgentBudget,
) -> Result<(), AppError> {
    if budget.max_children.is_some_and(|limit| {
        delegations
            .iter()
            .filter(|row| row.parent_run_id == run_id)
            .count()
            >= usize::from(limit)
    }) {
        return Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 已达到 max_children"),
            operation: "创建 child Agent".into(),
        });
    }
    if budget
        .max_delegation_depth
        .is_some_and(|limit| depth >= limit)
    {
        return Err(AppError::InvalidState {
            current: format!("Agent run {run_id} 已达到 max_delegation_depth"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

pub fn child_budget_within_parent(
    mut child: AgentBudget,
    parent: &AgentBudget,
    usage: &Value,
) -> Result<AgentBudget, AppError> {
    validate_agent_budget(&child)?;
    let steps_used = budget_usage_u32(usage, "steps_used")?;
    let model_calls_used = budget_usage_u32(usage, "model_calls_used")?;
    let tool_calls_used = budget_usage_u32(usage, "tool_calls_used")?;
    let tokens_used = budget_usage_u64(usage, "tokens_used")?;
    ensure_budget_remaining_u32(parent.max_steps, steps_used, "max_steps")?;
    ensure_budget_remaining_u32(parent.max_model_calls, model_calls_used, "max_model_calls")?;
    ensure_budget_remaining_u32(parent.max_tool_calls, tool_calls_used, "max_tool_calls")?;
    ensure_budget_remaining_u64(parent.max_tokens, tokens_used, "max_tokens")?;
    cap_budget_u32(&mut child.max_steps, parent.max_steps, steps_used);
    cap_budget_u32(
        &mut child.max_model_calls,
        parent.max_model_calls,
        model_calls_used,
    );
    cap_budget_u32(
        &mut child.max_tool_calls,
        parent.max_tool_calls,
        tool_calls_used,
    );
    cap_budget_u64(&mut child.max_tokens, parent.max_tokens, tokens_used);
    cap_budget_u64(
        &mut child.max_duration_seconds,
        parent.max_duration_seconds,
        0,
    );
    cap_budget_u16(&mut child.max_parallel_tools, parent.max_parallel_tools);
    cap_budget_u16(&mut child.max_children, parent.max_children);
    cap_budget_u8(
        &mut child.max_delegation_depth,
        parent
            .max_delegation_depth
            .map(|limit| limit.saturating_sub(1).max(1)),
    );
    Ok(child)
}

fn ensure_budget_remaining_u32(limit: Option<u32>, used: u32, field: &str) -> Result<(), AppError> {
    if limit.is_some_and(|limit| used >= limit) {
        return Err(AppError::InvalidState {
            current: format!("parent Agent budget {field} 已耗尽"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

fn ensure_budget_remaining_u64(limit: Option<u64>, used: u64, field: &str) -> Result<(), AppError> {
    if limit.is_some_and(|limit| used >= limit) {
        return Err(AppError::InvalidState {
            current: format!("parent Agent budget {field} 已耗尽"),
            operation: "创建 child Agent".into(),
        });
    }
    Ok(())
}

fn cap_budget_u32(child: &mut Option<u32>, parent: Option<u32>, used: u32) {
    if let Some(limit) = parent {
        let remaining = limit.saturating_sub(used).max(1);
        *child = Some(child.unwrap_or(remaining).min(remaining));
    }
}

fn cap_budget_u64(child: &mut Option<u64>, parent: Option<u64>, used: u64) {
    if let Some(limit) = parent {
        let remaining = limit.saturating_sub(used).max(1);
        *child = Some(child.unwrap_or(remaining).min(remaining));
    }
}

fn cap_budget_u16(child: &mut Option<u16>, parent: Option<u16>) {
    if let Some(limit) = parent {
        *child = Some(child.unwrap_or(limit).min(limit));
    }
}

fn cap_budget_u8(child: &mut Option<u8>, parent: Option<u8>) {
    if let Some(limit) = parent {
        *child = Some(child.unwrap_or(limit).min(limit));
    }
}

fn budget_usage_u32(value: &Value, key: &str) -> Result<u32, AppError> {
    u32::try_from(budget_usage_u64(value, key)?).map_err(|_| AppError::InvalidState {
        current: format!("Agent usage.{key} 超出 u32"),
        operation: "读取 Agent run budget usage".into(),
    })
}

fn budget_usage_u64(value: &Value, key: &str) -> Result<u64, AppError> {
    match value.get(key) {
        None => Ok(0),
        Some(value) => value.as_u64().ok_or_else(|| AppError::InvalidState {
            current: format!("Agent usage.{key} 不是非负整数"),
            operation: "读取 Agent run budget usage".into(),
        }),
    }
}

#[async_trait::async_trait]
pub trait AgentWorkerExecutionApplicationPort: Send + Sync {
    async fn worker_execution_by_key(
        &self,
        run_id: i64,
        key: &str,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn create_prepared_execution(
        &self,
        input: WorkerExecutionInput,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn update_worker_execution(
        &self,
        input: WorkerExecutionUpdate,
    ) -> Result<AgentExecutionSnapshot, AppError>;

    async fn worker_execution_by_key_request(
        &self,
        request: &AgentWorkerExecutionLookupRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn create_prepared_execution_request(
        &self,
        request: &AgentWorkerExecutionCreateRequest,
    ) -> Result<Option<AgentExecutionSnapshot>, AppError>;

    async fn update_worker_execution_request(
        &self,
        request: &AgentWorkerExecutionUpdateRequest,
    ) -> Result<AgentExecutionSnapshot, AppError>;
}

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

#[async_trait::async_trait]
pub trait AgentWorkerApprovalApplicationPort: Send + Sync {
    async fn approved_execution(&self, thread_id: i64, execution_id: i64)
    -> Result<bool, AppError>;

    async fn approved_execution_request(
        &self,
        request: &AgentWorkerApprovalCheckRequest,
    ) -> Result<AgentWorkerApprovalCheckSnapshotWire, AppError>;

    async fn request_worker_approval(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        operation_json: Value,
        resource_preconditions_json: Option<Value>,
        expires_at: Option<NaiveDateTime>,
    ) -> Result<(), AppError>;

    async fn request_worker_approval_request(
        &self,
        request: &AgentWorkerApprovalRequest,
    ) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerFailureApplicationPort: Send + Sync {
    async fn fail_child_run(
        &self,
        claimed: &AgentWorkerRun,
        error_message: String,
    ) -> Result<(), AppError>;

    async fn fail_worker_run(&self, request: &AgentWorkerFailureRequest) -> Result<(), AppError>;
}

#[async_trait::async_trait]
pub trait AgentWorkerMessageApplicationPort: Send + Sync {
    async fn append_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        execution_id: i64,
        tool_name: &str,
        tool_call_id: Option<&str>,
        result: Value,
    ) -> Result<AgentMessageSnapshot, AppError>;

    async fn append_assistant_tool_message(
        &self,
        claimed: &AgentWorkerRun,
        content: String,
        tool_calls: Value,
    ) -> Result<AgentMessageSnapshot, AppError>;

    async fn upsert_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        reasoning_content: Option<String>,
    ) -> Result<AgentMessageSnapshot, AppError>;

    async fn interrupt_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        reason: Option<String>,
    ) -> Result<(), AppError>;

    async fn complete_assistant_streaming_message(
        &self,
        claimed: &AgentWorkerRun,
        client_message_id: String,
        content: String,
        tool_calls: Value,
    ) -> Result<Option<AgentMessageSnapshot>, AppError>;

    async fn apply_message_request(
        &self,
        request: &AgentWorkerMessageRequest,
    ) -> Result<Option<AgentMessageSnapshot>, AppError>;
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_llm::provider::LlmTraceContext;
    use serde_json::json;

    #[test]
    fn child_budget_is_capped_by_parent_remaining_usage() {
        let parent = AgentBudget {
            max_steps: Some(10),
            max_model_calls: Some(8),
            max_tokens: Some(1_000),
            max_delegation_depth: Some(2),
            ..AgentBudget::default()
        };
        let child = AgentBudget {
            max_steps: Some(99),
            max_model_calls: Some(99),
            max_tokens: Some(9_999),
            max_delegation_depth: Some(9),
            ..AgentBudget::default()
        };

        let capped = child_budget_within_parent(
            child,
            &parent,
            &json!({
                "steps_used": 3,
                "model_calls_used": 2,
                "tokens_used": 250
            }),
        )
        .expect("child budget must be capped");
        assert_eq!(capped.max_steps, Some(7));
        assert_eq!(capped.max_model_calls, Some(6));
        assert_eq!(capped.max_tokens, Some(750));
        assert_eq!(capped.max_delegation_depth, Some(1));

        let exhausted = child_budget_within_parent(
            AgentBudget::default(),
            &AgentBudget {
                max_steps: Some(3),
                ..AgentBudget::default()
            },
            &json!({"steps_used": 3}),
        )
        .expect_err("an exhausted parent budget must reject another child");
        assert!(exhausted.to_string().contains("max_steps"));
    }

    #[test]
    fn context_summary_rejects_invalid_input() {
        let invalid = json!({
            "current_goal": "goal",
            "constraints": [],
            "decisions": [],
            "facts_resources": [],
            "completed": [],
            "pending": [],
            "tool_state": [],
            "covered_through_sequence": -1
        });
        assert!(parse_agent_context_summary(Some(&invalid.to_string())).is_err());

        let valid = AgentContextSummary {
            current_goal: "goal".into(),
            constraints: vec!["keep the contract stable".into()],
            decisions: Vec::new(),
            facts_resources: Vec::new(),
            completed: Vec::new(),
            pending: vec!["run the worker check".into()],
            tool_state: Vec::new(),
            covered_through_sequence: 4,
        };
        assert!(valid.validate().is_ok());
    }

    #[test]
    fn model_request_snapshot_round_trips_internal_fields() {
        let mut request = ChatRequest::system_user("system", "user", Some(0.2), Some(128));
        request.trace_context = Some(LlmTraceContext {
            project_id: Some(7),
            root_project_id: Some(3),
            workflow: "agent".into(),
            step_key: "model".into(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some("call-1".into()),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });
        request.context_manifest_json = Some(json!({"packs": ["thread"]}));

        let snapshot = model_request_snapshot(&request).expect("request must serialize");
        assert_eq!(snapshot["trace_context"]["project_id"], 7);
        assert_eq!(snapshot["context_manifest_json"]["packs"][0], "thread");

        let restored = restore_model_request_snapshot(snapshot).expect("snapshot must restore");
        assert_eq!(restored.messages.len(), 2);
        assert_eq!(restored.max_tokens, Some(128));
        assert_eq!(
            restored
                .trace_context
                .as_ref()
                .map(|trace| trace.project_id),
            Some(Some(7))
        );
        assert_eq!(
            restored.context_manifest_json,
            Some(json!({"packs": ["thread"]}))
        );
    }
}

pub use a4n_api_contract::agent::AgentEditorSource;
