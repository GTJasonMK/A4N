//! Worker-facing snapshots and in-memory inputs shared by Agent runtimes.

use super::*;

const MAX_CONTEXT_SUMMARY_BYTES: usize = 256 * 1024;
pub const AGENT_CONTEXT_SUMMARY_VERSION: u32 = 2;

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
    pub schema_version: u32,
    pub current_goal: String,
    pub constraints: Vec<String>,
    pub decisions: Vec<String>,
    pub facts_resources: Vec<String>,
    pub completed: Vec<String>,
    pub pending: Vec<String>,
    pub tool_state: Vec<String>,
    /// Durable message ID, never an event sequence or an estimated watermark.
    pub covered_through_sequence: i64,
}

impl AgentContextSummary {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.schema_version != AGENT_CONTEXT_SUMMARY_VERSION
            || self.covered_through_sequence < 0
            || self.current_goal.chars().count() > 4_000
        {
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
    pub client_message_id: Option<String>,
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

#[derive(Debug, Clone)]
pub struct AgentGenerationCancelSnapshot {
    pub task_id: String,
    pub attempt: u32,
    pub requested_by_user_id: i64,
    pub status: String,
    pub owner_stopped: bool,
}

#[derive(Debug, Clone)]
pub struct AgentWorkflowCancelSnapshot {
    pub workflow_run_id: i64,
    pub owner_stopped: bool,
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
            client_message_id: snapshot.client_message_id,
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
            client_message_id: snapshot.client_message_id,
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
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
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
