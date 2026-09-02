//! 全局 Agent 控制平面合同。
//!
//! Agent 只负责会话、编排、工具调用和委派；正文、画布、Workflow、Generation
//! 等领域数据仍由各自的合同和事实源负责。本模块因此只保存引用，不复制领域实体。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::runtime_config::RuntimeConfigSnapshot;

pub use a4n_domain::agent::{
    AgentApprovalDecision, AgentApprovalStatus, AgentControlAction, AgentDelegationStatus,
    AgentDeliveryMode, AgentExecutionKind, AgentExecutionMode, AgentExecutionStatus,
    AgentInboxKind, AgentInboxStatus, AgentMemoryScopeKind, AgentMemoryStatus, AgentMessageRole,
    AgentMessageStatus, AgentRunStatus, AgentThreadKind, AgentThreadStatus, AgentToolCostClass,
    AgentToolExposure, AgentToolReplayPolicy, AgentToolReversibility, AgentToolRisk,
    AgentTurnStatus, AgentTurnTrigger, AgentWaitingReason,
};

pub const AGENT_API_PROTOCOL_VERSION: u32 = 1;
pub const AGENT_EVENTS_DEFAULT_LIMIT: u32 = 100;
pub const AGENT_EVENTS_MAX_LIMIT: u32 = 1_000;
pub const AGENT_THREADS_DEFAULT_LIMIT: u32 = 50;
pub const AGENT_THREADS_MAX_LIMIT: u32 = 500;
pub const AGENT_TOOLS_DEFAULT_LIMIT: u32 = 50;
pub const AGENT_TOOLS_MAX_LIMIT: u32 = 500;
pub const MAIN_AGENT_CAPABILITY: &str = "main";
/// Generic user-created sub-agents use the same discovery/read foundation as
/// the main Agent, while still remaining children in the durable run tree.
/// Domain-specific capabilities add their own narrower catalogs below.
pub const GENERAL_AGENT_CAPABILITY: &str = "general";
pub const CONTENT_OPTIMIZATION_AGENT_CAPABILITY: &str = "content_optimization";
pub const CANVAS_AGENT_CAPABILITY: &str = "canvas";
pub const MAIN_AGENT_TOOLS: &[&str] = &[
    "search_agent_tools",
    "describe_agent_tool",
    "locate_writing_target",
    "list_projects",
    "get_project",
    "list_chapters",
    "delegate",
];
pub const CONTENT_OPTIMIZATION_AGENT_TOOLS: &[&str] = &[
    "search_agent_tools",
    "describe_agent_tool",
    "get_project",
    "list_chapters",
    "ensure_content_optimization_session",
    "send_content_optimization_message",
    "get_optimize_session",
    "list_optimize_turns",
    "list_optimize_patch_sets",
    "apply_optimization_patches",
    "get_generation_task_summary",
];
pub const CANVAS_AGENT_TOOLS: &[&str] = &[
    "search_agent_tools",
    "describe_agent_tool",
    "get_project",
    "get_canvas_graph",
    "get_canvas_node_contracts",
    "save_canvas_graph",
    "run_canvas_graph",
    "run_canvas_node",
    "get_workflow_run_detail",
];

/// Stable identity for a resource-bound sub-agent thread.
pub fn canonical_subagent_binding_key(
    capability: &str,
    resource_kind: &str,
    resource_id: &str,
) -> String {
    format!("subagent:{capability}:{resource_kind}:{resource_id}")
}

/// Whether a sub-agent capability is intentionally restricted to a fixed
/// domain catalog.  Generic/user-defined capabilities use the discovery
/// foundation first and may add any manifest-exposed Host command after the
/// model searches/describes it; keeping this distinction in the shared
/// contract prevents the admission and worker layers from drifting apart.
pub fn capability_has_restricted_tool_catalog(capability: &str) -> bool {
    matches!(
        capability.trim(),
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY | CANVAS_AGENT_CAPABILITY
    )
}

pub fn agent_tools_for_capability(capability: &str) -> &'static [&'static str] {
    match capability.trim() {
        MAIN_AGENT_CAPABILITY => MAIN_AGENT_TOOLS,
        GENERAL_AGENT_CAPABILITY => MAIN_AGENT_TOOLS,
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY => CONTENT_OPTIMIZATION_AGENT_TOOLS,
        CANVAS_AGENT_CAPABILITY => CANVAS_AGENT_TOOLS,
        // A manually-created profile may carry a user-defined capability
        // label. Keep it useful by granting only the generic discovery/read
        // foundation; domain adapters remain explicitly registered.
        _ => MAIN_AGENT_TOOLS,
    }
}

/// Descriptors for Agent-native tools and domain adapters that need an
/// explicit Agent policy even when their Host command carries a full runtime
/// snapshot. Keeping these descriptors in the shared contract lets the
/// model-facing runtime and tool-discovery API expose the same schemas and
/// safety policy.
pub fn synthetic_agent_tool_descriptor(
    name: &str,
) -> Option<crate::command_manifest::CommandDescriptor> {
    let domain = match name {
        "apply_optimization_patches" => "optimize",
        "run_canvas_graph" | "run_canvas_node" => "canvas",
        "get_generation_task_summary" => "task",
        "ensure_content_optimization_session" | "send_content_optimization_message" => "optimize",
        "search_agent_tools" | "describe_agent_tool" | "locate_writing_target" | "delegate" => {
            "agent"
        }
        _ => return None,
    };
    let inherited_schema = match name {
        "apply_optimization_patches" | "run_canvas_graph" | "run_canvas_node" => {
            Some(crate::command_manifest::command_descriptor(name)?)
        }
        _ => None,
    };
    let mut descriptor = crate::command_manifest::CommandDescriptor {
        name: name.to_string(),
        kind: "agent_tool".into(),
        domain: domain.into(),
        summary: String::new(),
        auth: "user".into(),
        effect: "write".into(),
        execution: "immediate".into(),
        args_schema: inherited_schema
            .as_ref()
            .map_or(Value::Null, |descriptor| descriptor.args_schema.clone()),
        result_schema: inherited_schema
            .as_ref()
            .map_or(Value::Null, |descriptor| descriptor.result_schema.clone()),
        secret_pointers: Vec::new(),
        file_inputs: Vec::new(),
        file_outputs: Vec::new(),
        result_references: Vec::new(),
        required_capabilities: Vec::new(),
        runtime_config_policy: crate::command_manifest::RuntimeConfigPolicy::None,
        idempotency: "none".into(),
        invoke: format!("internal agent tool: {name}"),
        page_scopes: Vec::new(),
        required_ui_context: Vec::new(),
        approval_policy: "agent_allowed".into(),
        navigation_hint: None,
        agent_policy: None,
    };
    descriptor.summary = match name {
        "apply_optimization_patches" => {
            "Apply reviewed content-optimization patches to the chapter".into()
        }
        "run_canvas_graph" => "Run a Canvas graph and return its workflow handle".into(),
        "run_canvas_node" => "Run one Canvas node and return its workflow handle".into(),
        "search_agent_tools" => {
            "Search the registered Host command catalog by name, domain, effect, or capability"
                .into()
        }
        "describe_agent_tool" => {
            "Read the complete schema and safety policy for one registered Host command".into()
        }
        "locate_writing_target" => {
            "Locate a recent writing project and its latest applicable chapter".into()
        }
        "delegate" => "Delegate a focused objective to a domain sub-agent".into(),
        "ensure_content_optimization_session" => {
            "Create or reuse the chapter's durable content-optimization session".into()
        }
        "send_content_optimization_message" => {
            "Send a focused instruction to the content-optimization Agent and return its generation task handle".into()
        }
        "get_generation_task_summary" => {
            "Read the real state and terminal result of an asynchronous generation task".into()
        }
        _ => unreachable!(),
    };
    descriptor.effect = match name {
        "search_agent_tools"
        | "describe_agent_tool"
        | "locate_writing_target"
        | "get_generation_task_summary" => "read".into(),
        _ => "write".into(),
    };
    descriptor.execution = match name {
        "delegate" => "agent".into(),
        "send_content_optimization_message" | "run_canvas_graph" | "run_canvas_node" => {
            "task".into()
        }
        _ => "immediate".into(),
    };
    descriptor.args_schema = match name {
        "apply_optimization_patches" | "run_canvas_graph" | "run_canvas_node" => {
            descriptor.args_schema.clone()
        }
        "search_agent_tools" => json!({
            "type": "object",
            "properties": {
                "query": { "type": ["string", "null"] },
                "domain": { "type": ["string", "null"] },
                "effect": { "type": ["string", "null"] },
                "capability": { "type": ["string", "null"] },
                "limit": { "type": ["integer", "null"], "minimum": 1, "maximum": 500 }
            },
            "additionalProperties": false
        }),
        "describe_agent_tool" => json!({
            "type": "object",
            "properties": { "name": { "type": "string", "minLength": 1 } },
            "required": ["name"],
            "additionalProperties": false
        }),
        "locate_writing_target" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": ["integer", "null"], "minimum": 1 },
                "chapterId": { "type": ["integer", "null"], "minimum": 1 },
                "includeEmpty": { "type": "boolean", "default": false }
            },
            "additionalProperties": false
        }),
        "delegate" => json!({
            "type": "object",
            "properties": {
                "capability": { "type": "string", "minLength": 1 },
                "task": { "type": "string", "minLength": 1 },
                "title": { "type": ["string", "null"], "maxLength": 200 },
                "projectId": { "type": ["integer", "null"], "minimum": 1 },
                "chapterId": { "type": ["integer", "null"], "minimum": 1 },
                "successCriteria": { "type": "array", "items": { "type": "string" } },
                "budget": { "type": ["object", "null"] }
            },
            "required": ["capability", "task"],
            "additionalProperties": false
        }),
        "ensure_content_optimization_session" => json!({
            "type": "object",
            "properties": {
                "projectId": { "type": "integer", "minimum": 1 },
                "chapterId": { "type": "integer", "minimum": 1 },
                "name": { "type": ["string", "null"], "minLength": 1, "maxLength": 200 }
            },
            "required": ["projectId", "chapterId"],
            "additionalProperties": false
        }),
        "send_content_optimization_message" => json!({
            "type": "object",
            "properties": {
                "sessionId": { "type": "integer", "minimum": 1 },
                "message": { "type": "string", "minLength": 1, "maxLength": 32768 }
            },
            "required": ["sessionId", "message"],
            "additionalProperties": false
        }),
        "get_generation_task_summary" => json!({
            "type": "object",
            "properties": {
                "taskId": { "type": "string", "minLength": 1 },
                "attempt": { "type": "integer", "minimum": 1 }
            },
            "required": ["taskId", "attempt"],
            "additionalProperties": false
        }),
        _ => unreachable!(),
    };
    descriptor.result_schema = match name {
        "apply_optimization_patches" | "run_canvas_graph" | "run_canvas_node" => {
            descriptor.result_schema.clone()
        }
        "search_agent_tools" => typed_schema::<AgentToolSearchResult>(),
        "describe_agent_tool" => typed_schema::<AgentToolDescriptor>(),
        "locate_writing_target" => typed_schema::<WritingTargetView>(),
        "delegate" => json!({
            "type": "object",
            "properties": {
                "delegation_id": { "type": "string", "minLength": 1 },
                "child_run_id": { "type": "string", "minLength": 1 },
                "result_refs": { "type": ["array", "null"] },
                "summary": { "type": ["string", "null"] },
                "cancelled": { "type": "boolean" },
                "recovery_required": { "type": "boolean" },
                "reason": { "type": ["string", "null"] }
            },
            "required": ["delegation_id", "child_run_id"],
            "additionalProperties": true,
            "x-a4n-schema-quality": "typed"
        }),
        "ensure_content_optimization_session" => {
            typed_schema::<crate::optimize::OptimizationSessionSummary>()
        }
        "send_content_optimization_message" => {
            typed_schema::<crate::generation_tasks::GenerationTaskHandle>()
        }
        "get_generation_task_summary" => {
            typed_schema::<crate::generation_tasks::GenerationTaskSummary>()
        }
        _ => unreachable!(),
    };
    let (exposure, risk, reversibility, replay_policy, tags) = match name {
        "delegate" => (
            AgentToolExposure::Preloaded,
            AgentToolRisk::Low,
            AgentToolReversibility::Reversible,
            AgentToolReplayPolicy::Idempotent,
            vec!["delegate".into(), "subagent".into()],
        ),
        "ensure_content_optimization_session" => (
            AgentToolExposure::Preloaded,
            AgentToolRisk::Low,
            AgentToolReversibility::Reversible,
            AgentToolReplayPolicy::Idempotent,
            vec!["optimize".into()],
        ),
        "send_content_optimization_message" => (
            AgentToolExposure::Preloaded,
            AgentToolRisk::Medium,
            AgentToolReversibility::Reversible,
            AgentToolReplayPolicy::ReconcileRequired,
            vec!["optimize".into(), "task".into()],
        ),
        "apply_optimization_patches" => (
            AgentToolExposure::Preloaded,
            AgentToolRisk::High,
            AgentToolReversibility::Compensatable,
            AgentToolReplayPolicy::ReconcileRequired,
            vec!["optimize".into(), "write".into()],
        ),
        "run_canvas_graph" | "run_canvas_node" => (
            AgentToolExposure::Preloaded,
            AgentToolRisk::High,
            AgentToolReversibility::Compensatable,
            AgentToolReplayPolicy::ReconcileRequired,
            vec!["canvas".into(), "workflow".into()],
        ),
        _ => (
            AgentToolExposure::Preloaded,
            AgentToolRisk::Low,
            AgentToolReversibility::Reversible,
            AgentToolReplayPolicy::Safe,
            vec!["agent".into()],
        ),
    };
    descriptor.agent_policy = Some(crate::command_manifest::AgentCommandPolicy {
        exposure,
        risk,
        reversibility,
        replay_policy,
        cost_class: if matches!(
            name,
            "send_content_optimization_message" | "run_canvas_graph" | "run_canvas_node"
        ) {
            AgentToolCostClass::Llm
        } else {
            AgentToolCostClass::None
        },
        tags,
    });
    Some(descriptor)
}

fn typed_schema<T: JsonSchema>() -> Value {
    let mut schema = serde_json::to_value(schemars::schema_for!(T))
        .expect("Agent tool result schema must serialize");
    schema
        .as_object_mut()
        .expect("Agent tool result schema must be an object")
        .insert("x-a4n-schema-quality".into(), Value::String("typed".into()));
    schema
}

pub fn synthetic_agent_tool_descriptors() -> Vec<crate::command_manifest::CommandDescriptor> {
    [
        "search_agent_tools",
        "describe_agent_tool",
        "locate_writing_target",
        "delegate",
        "apply_optimization_patches",
        "run_canvas_graph",
        "run_canvas_node",
        "ensure_content_optimization_session",
        "send_content_optimization_message",
        "get_generation_task_summary",
    ]
    .into_iter()
    .filter_map(synthetic_agent_tool_descriptor)
    .collect()
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentScope {
    #[serde(default)]
    pub resource_kind: Option<String>,
    #[serde(default)]
    pub resource_id: Option<String>,
    #[serde(default)]
    pub project_id: Option<i64>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub allowed_capabilities: Vec<String>,
    #[serde(default)]
    pub allowed_tools: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct AgentBudget {
    #[serde(default)]
    pub max_duration_seconds: Option<u64>,
    #[serde(default)]
    pub max_steps: Option<u32>,
    #[serde(default)]
    pub max_model_calls: Option<u32>,
    #[serde(default)]
    pub max_tool_calls: Option<u32>,
    #[serde(default)]
    pub max_tokens: Option<u64>,
    #[serde(default)]
    pub max_cost_micros: Option<u64>,
    #[serde(default)]
    pub max_parallel_tools: Option<u16>,
    #[serde(default)]
    pub max_delegation_depth: Option<u8>,
    #[serde(default)]
    pub max_children: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AgentResourceRef {
    pub kind: String,
    pub id: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentThreadView {
    pub protocol_version: u32,
    pub id: String,
    pub owner_user_id: i64,
    pub kind: AgentThreadKind,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bound_resource: Option<AgentResourceRef>,
    pub scope: AgentScope,
    #[serde(default)]
    pub default_model_config_id: Option<i64>,
    pub status: AgentThreadStatus,
    pub revision: u64,
    pub next_turn_sequence: u64,
    pub next_event_sequence: u64,
    #[serde(default)]
    pub context_summary: Option<Value>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentMessageView {
    pub id: String,
    pub thread_id: String,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    pub role: AgentMessageRole,
    pub status: AgentMessageStatus,
    pub content: String,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<Value>,
    pub created_at: String,
    #[serde(default)]
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentTurnView {
    pub id: String,
    pub thread_id: String,
    pub sequence: u64,
    pub trigger: AgentTurnTrigger,
    #[serde(default)]
    pub goal: Option<String>,
    pub status: AgentTurnStatus,
    #[serde(default)]
    pub final_message_id: Option<String>,
    #[serde(default)]
    pub result_refs: Vec<AgentResourceRef>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentRunView {
    pub id: String,
    pub thread_id: String,
    pub turn_id: String,
    pub attempt: u32,
    pub status: AgentRunStatus,
    #[serde(default)]
    pub waiting_reason: Option<AgentWaitingReason>,
    #[serde(default)]
    pub lease_owner: Option<String>,
    #[serde(default)]
    pub lease_expires_at: Option<String>,
    #[serde(default)]
    pub budget: AgentBudget,
    pub steps_used: u32,
    pub model_calls_used: u32,
    pub tool_calls_used: u32,
    pub tokens_used: u64,
    #[serde(default)]
    pub cost_micros: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentExecutionView {
    pub id: String,
    pub run_id: String,
    pub sequence: u64,
    pub kind: AgentExecutionKind,
    #[serde(default)]
    pub tool_name: Option<String>,
    pub status: AgentExecutionStatus,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub replay_policy: Option<AgentToolReplayPolicy>,
    #[serde(default)]
    pub request_digest: Option<String>,
    #[serde(default)]
    pub resource_refs: Vec<AgentResourceRef>,
    #[serde(default)]
    pub result_refs: Vec<AgentResourceRef>,
    #[serde(default)]
    pub result_summary: Option<String>,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentDelegationView {
    pub id: String,
    pub parent_run_id: String,
    #[serde(default)]
    pub parent_execution_id: Option<String>,
    pub child_thread_id: String,
    #[serde(default)]
    pub child_turn_id: Option<String>,
    #[serde(default)]
    pub child_run_id: Option<String>,
    pub capability: String,
    pub task: String,
    pub scope: AgentScope,
    pub status: AgentDelegationStatus,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub result_refs: Vec<AgentResourceRef>,
    #[serde(default)]
    pub acceptance_summary: Option<String>,
    #[serde(default)]
    pub acceptance_error: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentApprovalView {
    pub id: String,
    pub execution_id: String,
    pub status: AgentApprovalStatus,
    pub command: String,
    #[serde(default)]
    pub args_digest: Option<String>,
    #[serde(default)]
    pub resource_refs: Vec<AgentResourceRef>,
    #[serde(default)]
    pub resource_revision: Option<String>,
    #[serde(default)]
    pub expected_effect: Option<String>,
    #[serde(default)]
    pub expires_at: Option<String>,
    #[serde(default)]
    pub decided_by_user_id: Option<i64>,
    #[serde(default)]
    pub decision: Option<AgentApprovalDecision>,
    #[serde(default)]
    pub reason: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub decided_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentInboxItemView {
    pub id: String,
    pub thread_id: String,
    pub kind: AgentInboxKind,
    pub status: AgentInboxStatus,
    #[serde(default)]
    pub delivery_mode: Option<AgentDeliveryMode>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
    pub priority: i16,
    pub created_at: String,
    #[serde(default)]
    pub consumed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentMemoryView {
    pub id: String,
    pub scope_kind: AgentMemoryScopeKind,
    pub scope_id: String,
    pub status: AgentMemoryStatus,
    pub memory_kind: String,
    pub content: String,
    #[serde(default)]
    pub resource_refs: Vec<AgentResourceRef>,
    #[serde(default)]
    pub source_thread_id: Option<String>,
    #[serde(default)]
    pub source_message_id: Option<String>,
    #[serde(default)]
    pub supersedes_id: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentEvent {
    pub protocol_version: u32,
    pub thread_id: String,
    pub sequence: u64,
    pub kind: String,
    pub actor_kind: String,
    #[serde(default)]
    pub actor_id: Option<String>,
    #[serde(default)]
    pub turn_id: Option<String>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub execution_id: Option<String>,
    #[serde(default)]
    pub delegation_id: Option<String>,
    #[serde(default)]
    pub message_id: Option<String>,
    #[serde(default)]
    pub payload: Option<Value>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentThreadSnapshot {
    pub protocol_version: u32,
    pub thread: AgentThreadView,
    #[serde(default)]
    pub messages: Vec<AgentMessageView>,
    #[serde(default)]
    pub turns: Vec<AgentTurnView>,
    #[serde(default)]
    pub runs: Vec<AgentRunView>,
    #[serde(default)]
    pub executions: Vec<AgentExecutionView>,
    #[serde(default)]
    pub delegations: Vec<AgentDelegationView>,
    #[serde(default)]
    pub approvals: Vec<AgentApprovalView>,
    #[serde(default)]
    pub inbox: Vec<AgentInboxItemView>,
    #[serde(default)]
    pub memories: Vec<AgentMemoryView>,
    #[serde(default)]
    pub events: Vec<AgentEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentToolDescriptor {
    pub name: String,
    pub summary: String,
    pub domain: String,
    pub effect: String,
    pub exposure: AgentToolExposure,
    pub risk: AgentToolRisk,
    pub reversibility: AgentToolReversibility,
    pub replay_policy: AgentToolReplayPolicy,
    pub cost_class: AgentToolCostClass,
    #[serde(default)]
    pub tags: Vec<String>,
    pub args_schema: Value,
    pub result_schema: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentToolSearchResult {
    pub tools: Vec<AgentToolDescriptor>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct CreateAgentThreadInput {
    #[serde(default)]
    pub kind: AgentThreadKind,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bound_resource: Option<AgentResourceRef>,
    #[serde(default)]
    pub scope: AgentScope,
    #[serde(default)]
    pub default_model_config_id: Option<i64>,
}

impl Default for CreateAgentThreadInput {
    fn default() -> Self {
        Self {
            kind: AgentThreadKind::Main,
            capability: None,
            title: None,
            bound_resource: None,
            scope: AgentScope::default(),
            default_model_config_id: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct EnsureMainAgentThreadInput {
    #[serde(default)]
    pub title: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ForkAgentThreadInput {
    pub thread_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub bound_resource: Option<AgentResourceRef>,
    #[serde(default)]
    pub scope: Option<AgentScope>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct UpdateAgentThreadInput {
    pub thread_id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub default_model_config_id: Option<i64>,
    #[serde(default)]
    pub expected_revision: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArchiveAgentThreadInput {
    pub thread_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SendAgentMessageInput {
    pub thread_id: String,
    pub client_message_id: String,
    pub content: String,
    #[serde(default)]
    pub delivery_mode: AgentDeliveryMode,
    #[serde(default)]
    pub execution_mode: AgentExecutionMode,
    #[serde(default)]
    pub model_config_id: Option<i64>,
    #[serde(default)]
    pub budget: Option<AgentBudget>,
    /// 本次 Run 的完整运行时配置。Runtime 只在进程内按 Run 保存，不能持久化。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
}

impl Default for SendAgentMessageInput {
    fn default() -> Self {
        Self {
            thread_id: String::new(),
            client_message_id: String::new(),
            content: String::new(),
            delivery_mode: AgentDeliveryMode::Followup,
            execution_mode: AgentExecutionMode::Autonomous,
            model_config_id: None,
            budget: None,
            runtime_config_snapshot: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlAgentRunInput {
    pub thread_id: String,
    pub action: AgentControlAction,
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ResolveAgentApprovalInput {
    pub approval_id: String,
    pub decision: AgentApprovalDecision,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentEventsInput {
    pub thread_id: String,
    #[serde(default)]
    pub after_sequence: Option<u64>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListAgentThreadsInput {
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub include_archived: bool,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ListAgentMessagesInput {
    pub thread_id: String,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentRunInput {
    pub run_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchAgentToolsInput {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub effect: Option<String>,
    #[serde(default)]
    pub capability: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DescribeAgentToolInput {
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SpawnAgentInput {
    pub parent_run_id: String,
    pub capability: String,
    pub task: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub bound_resource: Option<AgentResourceRef>,
    #[serde(default)]
    pub scope: AgentScope,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    #[serde(default)]
    pub budget: Option<AgentBudget>,
    /// Full runtime configuration for a child Run created by a remote or
    /// desktop caller.  It is consumed by Runtime and never persisted.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct SendAgentMessageToChildInput {
    pub delegation_id: String,
    pub content: String,
    #[serde(default)]
    pub delivery_mode: AgentDeliveryMode,
    /// 创建新的 child Run 时使用的完整运行时配置。active Run 的 followup
    /// 不会替换其已经绑定的进程内快照。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub runtime_config_snapshot: Option<RuntimeConfigSnapshot>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ControlChildAgentInput {
    pub delegation_id: String,
    pub action: AgentControlAction,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentMemoryInput {
    pub scope_kind: AgentMemoryScopeKind,
    pub scope_id: String,
    pub memory_kind: String,
    pub content: String,
    #[serde(default)]
    pub resource_refs: Vec<AgentResourceRef>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RetractAgentMemoryInput {
    pub memory_id: String,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LocateWritingTargetInput {
    #[serde(default)]
    pub project_id: Option<i64>,
    #[serde(default)]
    pub chapter_id: Option<i64>,
    #[serde(default)]
    pub include_empty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct WritingTargetView {
    pub project: AgentResourceRef,
    pub chapter: AgentResourceRef,
    #[serde(default)]
    pub chapter_number: Option<i32>,
    #[serde(default)]
    pub content_revision: Option<String>,
    #[serde(default)]
    pub content_hash: Option<String>,
    pub selection_reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub struct AgentMessageAccepted {
    pub thread_id: String,
    pub message_id: String,
    pub turn_id: String,
    pub run_id: String,
    pub status: AgentRunStatus,
    /// `true` 仅表示本次消息创建了新的 durable Run。Runtime 以此决定
    /// 是否可为该 Run 首次绑定进程内配置快照；followup/steer/inject
    /// 不得替换已经运行中的快照。
    #[serde(default)]
    pub created_run: bool,
}

#[cfg(test)]
mod tests {
    use super::{
        CANVAS_AGENT_CAPABILITY, CONTENT_OPTIMIZATION_AGENT_CAPABILITY, GENERAL_AGENT_CAPABILITY,
        LocateWritingTargetInput, MAIN_AGENT_CAPABILITY, agent_tools_for_capability,
        synthetic_agent_tool_descriptor,
    };
    use crate::command_manifest::command_descriptor;

    #[test]
    fn capability_catalogs_are_non_empty_and_every_tool_is_resolvable() {
        for capability in [
            MAIN_AGENT_CAPABILITY,
            GENERAL_AGENT_CAPABILITY,
            CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
            CANVAS_AGENT_CAPABILITY,
        ] {
            let tools = agent_tools_for_capability(capability);
            assert!(!tools.is_empty(), "{capability} must expose tools");
            assert!(tools.contains(&"search_agent_tools"));
            assert!(tools.contains(&"describe_agent_tool"));

            for name in tools {
                let descriptor = synthetic_agent_tool_descriptor(name)
                    .or_else(|| command_descriptor(name))
                    .unwrap_or_else(|| panic!("{capability} tool {name} has no descriptor"));
                assert_eq!(descriptor.name, *name);
                assert!(
                    !descriptor.args_schema.is_null(),
                    "{name} args schema missing"
                );
                assert!(
                    !descriptor.result_schema.is_null(),
                    "{name} result schema missing"
                );
                assert_eq!(
                    descriptor.result_schema["x-a4n-schema-quality"], "typed",
                    "{name} must expose a typed result schema"
                );
                if synthetic_agent_tool_descriptor(name).is_some() {
                    assert_eq!(
                        descriptor.args_schema["additionalProperties"], false,
                        "synthetic tool {name} must reject unknown arguments"
                    );
                }
            }
        }
    }

    #[test]
    fn locate_writing_target_accepts_model_camel_case_arguments() {
        let input: LocateWritingTargetInput = serde_json::from_value(serde_json::json!({
            "projectId": 7,
            "chapterId": 11,
            "includeEmpty": true,
        }))
        .expect("camelCase Agent tool arguments should deserialize");
        assert_eq!(input.project_id, Some(7));
        assert_eq!(input.chapter_id, Some(11));
        assert!(input.include_empty);
        assert!(
            serde_json::from_value::<LocateWritingTargetInput>(serde_json::json!({
                "project_id": 7,
            }))
            .is_err()
        );
    }

    #[test]
    fn synthetic_descriptors_have_explicit_host_metadata() {
        let descriptor = synthetic_agent_tool_descriptor("locate_writing_target")
            .expect("locate tool must have a descriptor");
        assert_eq!(descriptor.kind, "agent_tool");
        assert_eq!(descriptor.auth, "user");
        assert!(descriptor.page_scopes.is_empty());
        assert!(descriptor.required_ui_context.is_empty());
        assert!(descriptor.navigation_hint.is_none());
        assert_eq!(
            descriptor.runtime_config_policy,
            crate::command_manifest::RuntimeConfigPolicy::None
        );
        assert_eq!(
            descriptor.invoke,
            "internal agent tool: locate_writing_target"
        );
    }
}
