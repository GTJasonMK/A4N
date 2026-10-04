use a4n_db_models::models::agent::{
    approval, delegation, event, execution, inbox, memory, message, run, thread, turn,
};
use chrono::NaiveDateTime;
use sea_orm::FromQueryResult;
use serde_json::Value;

macro_rules! record {
    ($name:ident, $module:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $(pub $field: $ty,)*
        }

        impl From<$module::Model> for $name {
            fn from(model: $module::Model) -> Self {
                Self { $($field: model.$field,)* }
            }
        }
    };
}

record!(AgentThreadRecord, thread {
    id: i64,
    owner_user_id: i64,
    thread_kind: String,
    profile_id: String,
    title: Option<String>,
    project_id: Option<i64>,
    resource_type: Option<String>,
    resource_id: Option<String>,
    canonical_binding_key: Option<String>,
    scope_json: Value,
    default_model_config_id: Option<i64>,
    status: String,
    is_default: bool,
    revision: i64,
    next_turn_sequence: i64,
    next_event_sequence: i64,
    context_summary: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    archived_at: Option<NaiveDateTime>,
});

record!(AgentMessageRecord, message {
    id: i64,
    thread_id: i64,
    turn_id: Option<i64>,
    run_id: Option<i64>,
    role: String,
    sender_kind: String,
    sender_thread_id: Option<i64>,
    client_message_id: Option<String>,
    content_sha256: String,
    content: String,
    status: String,
    payload_json: Option<Value>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
});

record!(AgentTurnRecord, turn {
    id: i64,
    thread_id: i64,
    sequence: i64,
    client_message_id: String,
    input_message_id: i64,
    final_message_id: Option<i64>,
    trigger: String,
    goal: Option<String>,
    status: String,
    execution_mode: String,
    model_override_json: Option<Value>,
    error_message: Option<String>,
    result_refs_json: Option<Value>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    completed_at: Option<NaiveDateTime>,
});

record!(AgentRunRecord, run {
    id: i64,
    thread_id: i64,
    turn_id: i64,
    root_run_id: Option<i64>,
    parent_run_id: Option<i64>,
    attempt: i32,
    status: String,
    waiting_reason: Option<String>,
    is_active: bool,
    next_execution_sequence: i64,
    budget_json: Value,
    usage_json: Value,
    worker_id: Option<String>,
    lease_token: Option<String>,
    lease_expires_at: Option<NaiveDateTime>,
    next_attempt_at: NaiveDateTime,
    last_error: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    started_at: Option<NaiveDateTime>,
    completed_at: Option<NaiveDateTime>,
});

record!(AgentExecutionRecord, execution {
    id: i64,
    run_id: i64,
    sequence: i64,
    execution_key: String,
    execution_kind: String,
    status: String,
    tool_name: Option<String>,
    effect: Option<String>,
    replay_policy: Option<String>,
    arguments_json: Option<Value>,
    arguments_sha256: Option<String>,
    resource_preconditions_json: Option<Value>,
    result_json: Option<Value>,
    result_summary: Option<String>,
    result_refs_json: Option<Value>,
    error_message: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    started_at: Option<NaiveDateTime>,
    completed_at: Option<NaiveDateTime>,
});

record!(AgentEventRecord, event {
    id: i64,
    thread_id: i64,
    sequence: i64,
    turn_id: Option<i64>,
    run_id: Option<i64>,
    execution_id: Option<i64>,
    delegation_id: Option<i64>,
    message_id: Option<i64>,
    actor_kind: String,
    actor_id: Option<String>,
    kind: String,
    status: String,
    public_text: String,
    payload_json: Option<Value>,
    created_at: NaiveDateTime,
});

record!(AgentInboxRecord, inbox {
    id: i64,
    thread_id: i64,
    turn_id: Option<i64>,
    run_id: Option<i64>,
    source_kind: String,
    source_thread_id: Option<i64>,
    inbox_kind: String,
    delivery_mode: String,
    priority: i32,
    dedupe_key: String,
    content_sha256: String,
    content: String,
    payload_json: Option<Value>,
    status: String,
    available_at: NaiveDateTime,
    claim_token: Option<String>,
    claim_expires_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    claimed_at: Option<NaiveDateTime>,
    consumed_at: Option<NaiveDateTime>,
});

record!(AgentDelegationRecord, delegation {
    id: i64,
    parent_thread_id: i64,
    parent_run_id: i64,
    parent_execution_id: Option<i64>,
    child_thread_id: i64,
    child_turn_id: Option<i64>,
    child_run_id: Option<i64>,
    delegation_key: String,
    capability: String,
    objective: String,
    success_criteria_json: Value,
    scope_json: Value,
    budget_json: Value,
    status: String,
    result_refs_json: Option<Value>,
    result_summary: Option<String>,
    error_message: Option<String>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    completed_at: Option<NaiveDateTime>,
});

record!(AgentApprovalRecord, approval {
    id: i64,
    thread_id: i64,
    run_id: i64,
    execution_id: i64,
    operation_digest: String,
    status: String,
    operation_json: Value,
    resource_preconditions_json: Option<Value>,
    decided_by_user_id: Option<i64>,
    decision_note: Option<String>,
    expires_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
    decided_at: Option<NaiveDateTime>,
    consumed_at: Option<NaiveDateTime>,
});

record!(AgentMemoryRecord, memory {
    id: i64,
    owner_user_id: i64,
    project_id: Option<i64>,
    thread_id: Option<i64>,
    scope_kind: String,
    scope_key: String,
    memory_key: String,
    revision: i64,
    status: String,
    content: String,
    content_json: Option<Value>,
    source_message_id: Option<i64>,
    source_execution_id: Option<i64>,
    supersedes_memory_id: Option<i64>,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    retracted_at: Option<NaiveDateTime>,
});

#[derive(Debug, Clone, PartialEq)]
pub struct AgentThreadHistoryRecord {
    pub thread: AgentThreadRecord,
    pub messages: Vec<AgentMessageRecord>,
    pub turns: Vec<AgentTurnRecord>,
    pub runs: Vec<AgentRunRecord>,
    pub executions: Vec<AgentExecutionRecord>,
    pub delegations: Vec<AgentDelegationRecord>,
    pub approvals: Vec<AgentApprovalRecord>,
    pub inbox: Vec<AgentInboxRecord>,
    pub memories: Vec<AgentMemoryRecord>,
    pub events: Vec<AgentEventRecord>,
}

#[derive(Debug, Clone)]
pub struct AgentThreadUpdateRecordInput {
    pub thread_id: i64,
    pub owner_user_id: i64,
    pub expected_revision: i64,
    pub scope_json: Option<Value>,
    pub title: Option<Option<String>>,
    pub default_model_config_id: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, FromQueryResult)]
pub struct AgentOwnedGenerationTaskRecord {
    pub task_id: String,
    pub attempt: i32,
    pub requested_by_user_id: i64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AgentExternalTaskWaitRecord {
    pub run: AgentRunRecord,
    pub thread: AgentThreadRecord,
    pub execution: AgentExecutionRecord,
}
