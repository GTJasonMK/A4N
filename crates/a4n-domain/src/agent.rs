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

#[derive(Debug, Clone, PartialEq)]
pub struct AgentDelegationReviewInput {
    pub user_id: i64,
    pub delegation_id: i64,
    pub expected_child_run_id: i64,
    pub status: String,
    pub reason: String,
    pub reviewed_refs: Vec<serde_json::Value>,
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
#[serde(deny_unknown_fields)]
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
    use super::AgentRunStatus;

    #[test]
    fn recovery_required_is_terminal_but_waiting_is_not() {
        assert!(AgentRunStatus::RecoveryRequired.is_terminal());
        assert!(!AgentRunStatus::Waiting.is_terminal());
    }
}
