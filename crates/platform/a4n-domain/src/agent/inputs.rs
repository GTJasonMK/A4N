use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentWorkflowOrigin {
    pub run_id: i64,
    pub execution_id: i64,
    pub lease_token: String,
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
