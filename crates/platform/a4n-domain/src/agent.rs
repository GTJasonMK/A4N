use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// Approved Agent execution identity used to bind a workflow to its starter.
/// Persistence validates the identity; the value itself is domain-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorkflowOrigin {
    pub run_id: i64,
    pub execution_id: i64,
    pub lease_token: String,
}

pub const AGENT_DEFAULT_MAX_STEPS: u32 = 32;
pub const AGENT_DEFAULT_MAX_TOOLS: u32 = 64;
pub const AGENT_PROTOCOL_NATIVE_TOOL_CALLING: &str = "native_tool_calling";
pub const AGENT_PROTOCOL_JSON_COMPAT: &str = "json_compat";

pub fn valid_agent_protocol(value: &str) -> bool {
    matches!(
        value.trim(),
        AGENT_PROTOCOL_NATIVE_TOOL_CALLING | AGENT_PROTOCOL_JSON_COMPAT
    )
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentDelegationReviewInput {
    pub user_id: i64,
    pub delegation_id: i64,
    pub expected_child_run_id: i64,
    pub status: String,
    pub reason: String,
    pub reviewed_refs: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentCompletionBlocker {
    pub kind: String,
    pub id: String,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct AgentThreadCreateInput {
    pub owner_user_id: i64,
    pub thread_kind: String,
    pub profile_id: String,
    pub title: Option<String>,
    pub project_id: Option<i64>,
    pub resource_type: Option<String>,
    pub resource_id: Option<String>,
    pub canonical_binding_key: Option<String>,
    pub scope_json: serde_json::Value,
    pub default_model_config_id: Option<i64>,
    pub is_default: bool,
}

#[derive(Debug, Clone)]
pub struct AgentMessageInput {
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub role: String,
    pub sender_kind: String,
    pub sender_thread_id: Option<i64>,
    pub client_message_id: Option<String>,
    pub content: String,
    pub status: String,
    pub payload_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AgentEventInput {
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub execution_id: Option<i64>,
    pub delegation_id: Option<i64>,
    pub message_id: Option<i64>,
    pub actor_kind: String,
    pub actor_id: Option<String>,
    pub kind: String,
    pub status: String,
    pub public_text: String,
    pub payload_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AgentInboxInput {
    pub thread_id: i64,
    pub turn_id: Option<i64>,
    pub run_id: Option<i64>,
    pub source_kind: String,
    pub source_thread_id: Option<i64>,
    pub inbox_kind: String,
    pub delivery_mode: String,
    pub priority: i32,
    pub dedupe_key: String,
    pub content: String,
    pub payload_json: Option<serde_json::Value>,
    pub available_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct AgentExecutionInput {
    pub run_id: i64,
    pub execution_key: String,
    pub execution_kind: String,
    pub status: String,
    pub tool_name: Option<String>,
    pub effect: Option<String>,
    pub replay_policy: Option<String>,
    pub arguments_json: Option<serde_json::Value>,
    pub resource_preconditions_json: Option<serde_json::Value>,
}

#[derive(Debug, Clone)]
pub struct AgentDelegationInput {
    pub parent_thread_id: i64,
    pub parent_run_id: i64,
    pub parent_execution_id: Option<i64>,
    pub child_thread_id: i64,
    pub child_turn_id: Option<i64>,
    pub child_run_id: Option<i64>,
    pub delegation_key: String,
    pub capability: String,
    pub objective: String,
    pub success_criteria_json: serde_json::Value,
    pub scope_json: serde_json::Value,
    pub budget_json: serde_json::Value,
    pub status: String,
}

#[derive(Debug, Clone)]
pub struct AgentMemoryInput {
    pub owner_user_id: i64,
    pub project_id: Option<i64>,
    pub thread_id: Option<i64>,
    pub scope_kind: String,
    pub scope_key: String,
    pub memory_key: String,
    pub revision: i64,
    pub status: String,
    pub content: String,
    pub content_json: Option<serde_json::Value>,
    pub source_message_id: Option<i64>,
    pub source_execution_id: Option<i64>,
    pub supersedes_memory_id: Option<i64>,
}

#[derive(Debug, Clone)]
pub struct AgentApprovalInput {
    pub thread_id: i64,
    pub run_id: i64,
    pub execution_id: i64,
    pub operation_digest: String,
    pub operation_json: serde_json::Value,
    pub resource_preconditions_json: Option<serde_json::Value>,
    pub expires_at: Option<NaiveDateTime>,
}

#[derive(Debug, Clone)]
pub struct AgentTurnInput {
    pub thread_id: i64,
    pub client_message_id: String,
    pub message: AgentMessageInput,
    pub execution_mode: String,
    pub model_override_json: Option<serde_json::Value>,
    pub budget_json: serde_json::Value,
    pub runtime_snapshot_required: bool,
    pub trigger: String,
    pub goal: Option<String>,
}

/// Reserve conservatively using UTF-8 bytes plus output and framing allowance.
/// Actual provider usage replaces this reservation when the call completes.
pub fn agent_model_token_reservation(request: &serde_json::Value) -> u64 {
    serde_json::to_vec(request)
        .expect("JSON Value serializes")
        .len() as u64
        + request
            .get("max_tokens")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
        + 1024
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct AgentBudget {
    pub max_duration_seconds: Option<u64>,
    pub max_steps: Option<u32>,
    pub max_model_calls: Option<u32>,
    pub max_tool_calls: Option<u32>,
    pub max_tokens: Option<u64>,
    pub max_cost_micros: Option<u64>,
    pub max_parallel_tools: Option<u16>,
    pub max_delegation_depth: Option<u8>,
    pub max_children: Option<u16>,
}

impl AgentBudget {
    pub fn effective_for_run(&self, root: bool) -> Self {
        let mut effective = self.clone();
        if root {
            effective.max_steps = Some(self.max_steps.unwrap_or(AGENT_DEFAULT_MAX_STEPS));
            effective.max_model_calls =
                Some(self.max_model_calls.unwrap_or(AGENT_DEFAULT_MAX_STEPS));
            effective.max_tool_calls = Some(self.max_tool_calls.unwrap_or(AGENT_DEFAULT_MAX_TOOLS));
        }
        effective
    }
    pub fn validate(&self) -> Result<(), String> {
        if [
            self.max_duration_seconds,
            self.max_tokens,
            self.max_cost_micros,
        ]
        .into_iter()
        .flatten()
        .any(|value| value == 0)
            || [self.max_steps, self.max_model_calls, self.max_tool_calls]
                .into_iter()
                .flatten()
                .any(|value| value == 0)
            || [self.max_parallel_tools, self.max_children]
                .into_iter()
                .flatten()
                .any(|value| value == 0)
            || self.max_delegation_depth == Some(0)
        {
            return Err("Agent budget 上限必须为正数".into());
        }
        if self.max_duration_seconds.is_some_and(|value| {
            i64::try_from(value)
                .ok()
                .and_then(chrono::Duration::try_seconds)
                .and_then(|duration| chrono::Utc::now().checked_add_signed(duration))
                .is_none()
                || std::time::Instant::now()
                    .checked_add(std::time::Duration::from_secs(value))
                    .is_none()
        }) {
            return Err("Agent max_duration_seconds 超出时间实现上限".into());
        }
        if self.max_cost_micros.is_some() {
            return Err(
                "当前 provider 没有可靠的 Agent cost usage，暂不支持 max_cost_micros".into(),
            );
        }
        Ok(())
    }

    /// Values are absolute new ceilings; omission never removes an existing cap.
    pub fn extended(&self, extension: &Self, root: bool) -> Result<Self, String> {
        self.validate()?;
        extension.validate()?;
        if extension == &Self::default() {
            return Err("请至少提供一个新的 Agent 预算上限".into());
        }
        let mut next = self.effective_for_run(root);
        extend_budget_limit(
            &mut next.max_duration_seconds,
            extension.max_duration_seconds,
            None,
            "max_duration_seconds",
        )?;
        extend_budget_limit(&mut next.max_steps, extension.max_steps, None, "max_steps")?;
        extend_budget_limit(
            &mut next.max_model_calls,
            extension.max_model_calls,
            None,
            "max_model_calls",
        )?;
        extend_budget_limit(
            &mut next.max_tool_calls,
            extension.max_tool_calls,
            None,
            "max_tool_calls",
        )?;
        extend_budget_limit(
            &mut next.max_tokens,
            extension.max_tokens,
            None,
            "max_tokens",
        )?;
        extend_budget_limit(
            &mut next.max_parallel_tools,
            extension.max_parallel_tools,
            Some(1),
            "max_parallel_tools",
        )?;
        extend_budget_limit(
            &mut next.max_delegation_depth,
            extension.max_delegation_depth,
            None,
            "max_delegation_depth",
        )?;
        extend_budget_limit(
            &mut next.max_children,
            extension.max_children,
            None,
            "max_children",
        )?;
        Ok(next)
    }
}

fn extend_budget_limit<T: Copy + Ord>(
    current: &mut Option<T>,
    requested: Option<T>,
    default: Option<T>,
    field: &str,
) -> Result<(), String> {
    let Some(requested) = requested else {
        return Ok(());
    };
    if current.or(default).is_none_or(|limit| requested < limit) {
        return Err(format!("续做不能缩小 Agent {field} 上限"));
    }
    *current = Some(requested);
    Ok(())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentThreadKind {
    #[default]
    Main,
    Subagent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentThreadStatus {
    Active,
    Archived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageRole {
    User,
    Assistant,
    System,
    Tool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMessageStatus {
    Streaming,
    Complete,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentTurnTrigger {
    User,
    Delegation,
    Recovery,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentTurnStatus {
    Queued,
    Active,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentRunStatus {
    Queued,
    Running,
    Waiting,
    Pausing,
    Paused,
    Cancelling,
    Cancelled,
    Completed,
    Failed,
    Interrupted,
    RecoveryRequired,
}

impl AgentRunStatus {
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Cancelled
                | Self::Completed
                | Self::Failed
                | Self::Interrupted
                | Self::RecoveryRequired
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentWaitingReason {
    User,
    Approval,
    Child,
    External,
    Backoff,
    Budget,
    Reconcile,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionKind {
    ModelCall,
    ToolCall,
    Wait,
    Delegation,
    Verification,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionStatus {
    Prepared,
    AwaitingUser,
    Dispatching,
    Running,
    Succeeded,
    Failed,
    Rejected,
    Cancelled,
    OutcomeUnknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentDeliveryMode {
    #[default]
    Followup,
    Steer,
    Inject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentExecutionMode {
    #[default]
    Autonomous,
    ReviewWrites,
    AdviceOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentControlAction {
    Pause,
    Resume,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentInboxKind {
    Followup,
    Steer,
    Inject,
    Control,
    ChildResult,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentInboxStatus {
    Pending,
    Claimed,
    Consumed,
    Superseded,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentDelegationStatus {
    Pending,
    Running,
    AwaitingAcceptance,
    RevisionRequested,
    Accepted,
    Failed,
    Cancelled,
    Interrupted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentApprovalStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Consumed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentApprovalDecision {
    Approve,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryScopeKind {
    User,
    Project,
    Thread,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentMemoryStatus {
    Proposed,
    Active,
    Superseded,
    Retracted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolExposure {
    Hidden,
    Discoverable,
    Preloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolRisk {
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolReversibility {
    Reversible,
    Compensatable,
    Irreversible,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolReplayPolicy {
    Safe,
    Idempotent,
    ReconcileRequired,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AgentToolCostClass {
    None,
    Llm,
    Image,
    Video,
    ExternalUnknown,
}

#[cfg(test)]
mod tests {
    use super::{AgentBudget, AgentRunStatus};

    #[test]
    fn duration_budget_must_fit_wall_and_monotonic_clocks() {
        for limit in [u64::MAX, i64::MAX as u64, (i64::MAX / 1000) as u64] {
            let budget = AgentBudget {
                max_duration_seconds: Some(limit),
                ..Default::default()
            };
            assert!(
                budget
                    .validate()
                    .unwrap_err()
                    .contains("max_duration_seconds")
            );
        }
        for limit in [None, Some(1), Some(86_400)] {
            AgentBudget {
                max_duration_seconds: limit,
                ..Default::default()
            }
            .validate()
            .unwrap();
        }
    }

    #[test]
    fn recovery_required_is_terminal_but_waiting_is_not() {
        assert!(AgentRunStatus::RecoveryRequired.is_terminal());
        assert!(!AgentRunStatus::Waiting.is_terminal());
    }
}
