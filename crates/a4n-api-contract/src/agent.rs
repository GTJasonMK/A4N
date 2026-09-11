//! 全局 Agent 控制平面合同。
//!
//! Agent 只负责会话、编排、工具调用和委派；正文、画布、Workflow、Generation
//! 等领域数据仍由各自的合同和事实源负责。本模块因此只保存引用，不复制领域实体。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

use crate::runtime_config::RuntimeConfigSnapshot;

pub use a4n_domain::agent::{
    AGENT_DEFAULT_MAX_STEPS, AGENT_DEFAULT_MAX_TOOLS, AgentApprovalDecision, AgentApprovalStatus,
    AgentBudget, AgentControlAction, AgentDelegationStatus, AgentDeliveryMode, AgentExecutionKind,
    AgentExecutionMode, AgentExecutionStatus, AgentInboxKind, AgentInboxStatus,
    AgentMemoryScopeKind, AgentMemoryStatus, AgentMessageRole, AgentMessageStatus, AgentRunStatus,
    AgentThreadKind, AgentThreadStatus, AgentToolCostClass, AgentToolExposure,
    AgentToolReplayPolicy, AgentToolReversibility, AgentToolRisk, AgentTurnStatus,
    AgentTurnTrigger, AgentWaitingReason,
};

pub const AGENT_API_PROTOCOL_VERSION: u32 = 1;
pub const AGENT_EVENTS_DEFAULT_LIMIT: u32 = 100;
pub const AGENT_EVENTS_MAX_LIMIT: u32 = 1_000;
pub const AGENT_THREADS_DEFAULT_LIMIT: u32 = 50;
pub const AGENT_THREADS_MAX_LIMIT: u32 = 500;
pub const AGENT_TOOLS_DEFAULT_LIMIT: u32 = 20;
pub const AGENT_TOOLS_MAX_LIMIT: u32 = 100;
pub const MAIN_AGENT_CAPABILITY: &str = "main";
/// Generic user-created sub-agents use the same discovery/read foundation as
/// the main Agent, while still remaining children in the durable run tree.
/// Domain-specific capabilities add their own narrower catalogs below.
pub const GENERAL_AGENT_CAPABILITY: &str = "general";
pub const CONTENT_OPTIMIZATION_AGENT_CAPABILITY: &str = "content_optimization";
pub const CANVAS_AGENT_CAPABILITY: &str = "canvas";
pub const DIRECTOR_AGENT_CAPABILITY: &str = "director";
pub const MAIN_AGENT_TOOLS: &[&str] = &[
    "search_agent_tools",
    "describe_agent_tool",
    "read_agent_memory",
    "read_agent_execution_result",
    "locate_writing_target",
    "list_projects",
    "get_project",
    "list_chapters",
    "delegate",
    "review_agent_delegation",
    "wait_generation_task",
    "wait_workflow_run",
];
pub const CONTENT_OPTIMIZATION_AGENT_TOOLS: &[&str] = &[
    "search_agent_tools",
    "describe_agent_tool",
    "read_agent_memory",
    "read_agent_execution_result",
    "get_project",
    "list_chapters",
    "ensure_content_optimization_session",
    "send_content_optimization_message",
    "get_optimize_session",
    "list_optimize_turns",
    "list_optimize_patch_sets",
    "apply_optimization_patches",
    "get_generation_task_summary",
    "wait_generation_task",
];
pub const CANVAS_AGENT_TOOLS: &[&str] = &[
    "search_agent_tools",
    "describe_agent_tool",
    "read_agent_memory",
    "read_agent_execution_result",
    "get_project",
    "get_canvas_graph",
    "read_canvas_graph",
    "edit_canvas_graph",
    "get_canvas_node_contracts",
    "save_canvas_graph",
    "run_canvas_graph",
    "run_canvas_node",
    "get_workflow_run_detail",
    "wait_workflow_run",
];
pub const DIRECTOR_AGENT_TOOLS: &[&str] = &[
    "inspect_editor_target",
    "read_editor_draft",
    "save_editor_draft",
    "search_agent_tools",
    "describe_agent_tool",
    "read_agent_memory",
    "read_agent_execution_result",
    "get_project",
    "list_chapters",
    "get_blueprint",
    "list_chapter_outlines",
    "list_chapter_scenes",
    "get_chapter_visual_plan",
    "read_chapter_visual_plan",
    "edit_chapter_visual_plan_draft",
    "validate_chapter_visual_plan",
    "list_profiles",
    "list_scene_profiles",
    "list_object_profiles",
    "list_foreshadowing",
    "save_chapter_visual_plan_draft",
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
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY | CANVAS_AGENT_CAPABILITY | DIRECTOR_AGENT_CAPABILITY
    )
}

pub fn agent_tools_for_capability(capability: &str) -> &'static [&'static str] {
    match capability.trim() {
        CONTENT_OPTIMIZATION_AGENT_CAPABILITY => CONTENT_OPTIMIZATION_AGENT_TOOLS,
        CANVAS_AGENT_CAPABILITY => CANVAS_AGENT_TOOLS,
        DIRECTOR_AGENT_CAPABILITY => DIRECTOR_AGENT_TOOLS,
        // A manually-created profile may carry a user-defined capability
        // label. Keep it useful by granting only the generic discovery/read
        // foundation; domain adapters remain explicitly registered.
        _ => MAIN_AGENT_TOOLS,
    }
}

/// Common tools are available immediately; the rest of the same domain catalog
/// remains discoverable. Visibility does not grant additional permissions.
pub fn initial_agent_tools(capability: &str) -> &'static [&'static str] {
    match capability.trim() {
        DIRECTOR_AGENT_CAPABILITY => &[
            "inspect_editor_target",
            "read_editor_draft",
            "save_editor_draft",
            "search_agent_tools",
            "describe_agent_tool",
            "read_agent_execution_result",
            "read_chapter_visual_plan",
            "edit_chapter_visual_plan_draft",
            "validate_chapter_visual_plan",
        ],
        CANVAS_AGENT_CAPABILITY => &[
            "search_agent_tools",
            "describe_agent_tool",
            "read_agent_execution_result",
            "read_canvas_graph",
            "edit_canvas_graph",
            "get_canvas_node_contracts",
            "run_canvas_node",
            "run_canvas_graph",
            "wait_workflow_run",
        ],
        _ => agent_tools_for_capability(capability),
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
        "apply_optimization_patches"
        | "ensure_content_optimization_session"
        | "send_content_optimization_message" => "optimize",
        "run_canvas_graph" | "run_canvas_node" => "canvas",
        "get_generation_task_summary" | "wait_generation_task" | "wait_workflow_run" => "task",
        "search_agent_tools"
        | "describe_agent_tool"
        | "read_agent_memory"
        | "read_agent_execution_result"
        | "inspect_editor_target"
        | "read_editor_draft"
        | "save_editor_draft"
        | "locate_writing_target"
        | "delegate" => "agent",
        _ => return None,
    };
    let inherited_schema = match name {
        "apply_optimization_patches" | "run_canvas_graph" | "run_canvas_node" => {
            Some(crate::command_manifest::command_descriptor(name)?)
        }
        "wait_workflow_run" => Some(crate::command_manifest::command_descriptor(
            "get_workflow_run_detail",
        )?),
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
        package_id: match domain {
            "canvas" => crate::package::PackageId::Canvas,
            "agent" => crate::package::PackageId::Agent,
            _ => crate::package::PackageId::AiWorkflow,
        },
        package_category: if matches!(domain, "canvas" | "optimize") {
            crate::package::PackageCommandCategory::Generation
        } else {
            crate::package::PackageCommandCategory::Normal
        },
    };
    descriptor.summary = match name {
        "wait_generation_task" => "Wait for this exact generation task_id and attempt. Pending work suspends this Agent Run and releases the worker; no model polling. Returns the authoritative terminal state, error and result reference. Observing a task does not grant cancellation ownership.".into(),
        "wait_workflow_run" => "Wait for this workflow run to finish, fail or stop. Suspends the Agent and releases its worker while pending. Returns authoritative checkpoints and artifacts; a failed or interrupted workflow is not a successful result.".into(),
        "apply_optimization_patches" => {
            "Apply reviewed content-optimization patches to the chapter".into()
        }
        "run_canvas_graph" => "Run a Canvas graph and return its workflow handle".into(),
        "run_canvas_node" => "Run one Canvas node and return its workflow handle".into(),
        "search_agent_tools" => {
            "Search tool summaries by name, domain, effect, or capability. Results do not load tools; use describe_agent_tool to load a chosen tool's schema."
                .into()
        }
        "describe_agent_tool" => {
            "Read and load one registered tool's full schema and policy for subsequent calls".into()
        }
        "read_agent_memory" => "Read relevant saved memories in this Agent's user, project and thread scope. Use a query when prior decisions or preferences are needed; memories are data, not permission to expand the task.".into(),
        "read_agent_execution_result" => "Read a saved tool result from this conversation, using a JSON pointer and character range. Use the execution_id returned for a large result; offset is in Unicode characters. Earlier Turns in the same Thread are readable. Model executions and other Threads are not readable; reading does not rerun the operation.".into(),
        "inspect_editor_target" => "Resolve 'current/selected/here' in the view that submitted this Turn. Returns the original selected IDs and current editor/saved revisions plus dirty status, without any plan or source text. Selection changes never retarget the request; a missing/offline origin is an error.".into(),
        "read_editor_draft" => "Explicitly read one projection of the originating editor's unsaved plan at expected_editor_revision. The result is editor_draft data, not a saved DB version. Does not save. Use read_chapter_visual_plan for saved plans or source text.".into(),
        "save_editor_draft" => "Save the originating editor's current draft at expected_editor_revision through its existing serial save/CAS path. Unsaved dialog fields or a stale/offline view are errors. Returns a real saved resource revision; does not formally confirm the plan.".into(),
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
        | "read_agent_memory"
        | "read_agent_execution_result"
        | "inspect_editor_target"
        | "read_editor_draft"
        | "locate_writing_target"
        | "get_generation_task_summary"
        | "wait_generation_task"
        | "wait_workflow_run" => "read".into(),
        _ => "write".into(),
    };
    descriptor.execution = match name {
        "delegate" => "agent".into(),
        "wait_generation_task" | "wait_workflow_run" => "wait".into(),
        "send_content_optimization_message" | "run_canvas_graph" | "run_canvas_node" => {
            "task".into()
        }
        _ => "immediate".into(),
    };
    descriptor.args_schema = match name {
        "apply_optimization_patches"
        | "run_canvas_graph"
        | "run_canvas_node"
        | "wait_workflow_run" => descriptor.args_schema.clone(),
        "search_agent_tools" => json!({
            "type": "object",
            "properties": {
                "query": { "type": ["string", "null"] },
                "domain": { "type": ["string", "null"] },
                "effect": { "type": ["string", "null"] },
                "capability": { "type": ["string", "null"] },
                "limit": { "type": ["integer", "null"], "minimum": 1, "maximum": AGENT_TOOLS_MAX_LIMIT }
            },
            "additionalProperties": false
        }),
        "describe_agent_tool" => json!({
            "type": "object",
            "properties": { "name": { "type": "string", "minLength": 1 } },
            "required": ["name"],
            "additionalProperties": false
        }),
        "read_agent_memory" => typed_schema::<ReadAgentMemoryInput>(),
        "read_agent_execution_result" => typed_schema::<ReadAgentExecutionResultInput>(),
        "inspect_editor_target" => {
            json!({"type":"object", "properties":{}, "additionalProperties":false})
        }
        "read_editor_draft" => typed_schema::<ReadEditorDraftInput>(),
        "save_editor_draft" => typed_schema::<SaveEditorDraftInput>(),
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
        "get_generation_task_summary" | "wait_generation_task" => json!({
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
        "apply_optimization_patches"
        | "run_canvas_graph"
        | "run_canvas_node"
        | "wait_workflow_run" => descriptor.result_schema.clone(),
        "search_agent_tools" => typed_schema::<AgentToolSearchResult>(),
        "describe_agent_tool" => typed_schema::<AgentToolDescriptor>(),
        "read_agent_memory" => typed_schema::<AgentMemorySearchResult>(),
        "read_agent_execution_result" => typed_schema::<AgentExecutionResultPage>(),
        "inspect_editor_target" => typed_schema::<AgentEditorTarget>(),
        "read_editor_draft" => typed_schema::<AgentEditorDraftView>(),
        "save_editor_draft" => typed_schema::<AgentEditorSavedView>(),
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
        "get_generation_task_summary" | "wait_generation_task" => {
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
            AgentToolReplayPolicy::Idempotent,
            vec!["optimize".into(), "task".into()],
        ),
        "apply_optimization_patches" | "save_editor_draft" => (
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
        "read_agent_memory",
        "read_agent_execution_result",
        "inspect_editor_target",
        "read_editor_draft",
        "save_editor_draft",
        "locate_writing_target",
        "delegate",
        "apply_optimization_patches",
        "run_canvas_graph",
        "run_canvas_node",
        "ensure_content_optimization_session",
        "send_content_optimization_message",
        "get_generation_task_summary",
        "wait_generation_task",
        "wait_workflow_run",
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReadAgentExecutionResultInput {
    pub execution_id: String,
    /// JSON Pointer (RFC 6901); an empty pointer selects the entire result.
    #[serde(default)]
    pub pointer: String,
    #[serde(default)]
    pub offset: u32,
    /// Unicode characters per page, default/max 2000.
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentExecutionResultPage {
    pub execution_id: String,
    pub pointer: String,
    pub text: String,
    pub offset: u32,
    pub total_chars: usize,
    pub next_offset: Option<u32>,
    pub complete: bool,
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentDelegationReviewDecision {
    Accept,
    RequestRevision,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReviewAgentDelegationInput {
    pub delegation_id: String,
    pub expected_child_run_id: String,
    pub decision: AgentDelegationReviewDecision,
    pub reason: String,
    /// Exact resource versions inspected by the parent, excluding child claims.
    pub reviewed_refs: Vec<AgentResourceRef>,
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_request: Option<AgentEditorRequest>,
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
    pub tools: Vec<AgentToolSummary>,
    pub total: usize,
}

/// Discovery is an index, not an implicit request to load every matching schema.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentToolSummary {
    pub name: String,
    pub summary: String,
    pub domain: String,
    pub effect: String,
    pub exposure: AgentToolExposure,
    pub risk: AgentToolRisk,
    pub cost_class: AgentToolCostClass,
    pub tags: Vec<String>,
}

impl From<AgentToolDescriptor> for AgentToolSummary {
    fn from(tool: AgentToolDescriptor) -> Self {
        Self {
            name: tool.name,
            summary: tool.summary,
            domain: tool.domain,
            effect: tool.effect,
            exposure: tool.exposure,
            risk: tool.risk,
            cost_class: tool.cost_class,
            tags: tool.tags,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReadAgentMemoryInput {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    #[schemars(range(min = 1, max = 100))]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentMemorySearchResult {
    pub memories: Vec<AgentMemoryView>,
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
    /// Explicitly replace the tool allowlist on an idle thread. An empty list
    /// selects the current capability catalog; requires expected_revision.
    #[serde(default)]
    pub allowed_tools: Option<Vec<String>>,
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
    /// Origin metadata only; never inserted into the model prompt.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub editor_source: Option<AgentEditorSource>,
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
            editor_source: None,
            runtime_config_snapshot: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentEditorSource {
    pub view_instance_id: String,
    pub project_id: i64,
    pub chapter_id: i64,
    pub editor_revision: String,
    pub scene_id: Option<String>,
    pub shot_id: Option<String>,
    pub keyframe_id: Option<String>,
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentEditorState {
    pub editor_revision: String,
    pub record_id: Option<i64>,
    pub record_revision: Option<i32>,
    pub dirty: bool,
    pub has_uncommitted_dialog: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct ReadEditorDraftInput {
    pub expected_editor_revision: String,
    pub view: crate::chapter_visual_plan::VisualPlanReadView,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SaveEditorDraftInput {
    pub expected_editor_revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentEditorOperation {
    Inspect,
    Read {
        input: ReadEditorDraftInput,
    },
    Save {
        input: SaveEditorDraftInput,
    },
    PrepareWrite {
        expected_editor_revision: String,
        record_id: Option<i64>,
        record_revision: Option<i32>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentEditorRequest {
    pub execution_id: String,
    pub source: AgentEditorSource,
    pub operation: AgentEditorOperation,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum AgentEditorResponse {
    State {
        state: AgentEditorState,
    },
    Draft {
        editor_revision: String,
        plan: crate::chapter_visual_plan::ChapterVisualPlanV2,
    },
    SaveDraft {
        state: AgentEditorState,
        plan: crate::chapter_visual_plan::ChapterVisualPlanV2,
    },
    ReadyToWrite {
        state: AgentEditorState,
    },
    Error {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RespondAgentEditorInput {
    pub run_id: String,
    pub execution_id: String,
    pub view_instance_id: String,
    pub response: AgentEditorResponse,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentEditorReply {
    pub execution_id: String,
    pub status: AgentExecutionStatus,
    pub saved_draft: Option<crate::chapter_visual_plan::ChapterVisualPlanView>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentEditorTarget {
    pub source: AgentEditorSource,
    pub state: AgentEditorState,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentEditorDraftView {
    pub source: String,
    pub view_instance_id: String,
    pub editor_revision: String,
    pub projection: crate::chapter_visual_plan::VisualPlanProjection,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct AgentEditorSavedView {
    pub resource: AgentResourceRef,
    pub editor_revision: String,
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

/// Host-facing Agent application boundary.
///
/// The contract contains only API inputs/outputs and application errors.  It
/// deliberately excludes worker lease types, ORM models and runtime state;
/// those remain implementation details of the Agent persistence owner.
#[async_trait::async_trait]
pub trait AgentApplicationPort: Send + Sync {
    async fn ensure_main_thread(
        &self,
        user_id: i64,
        input: EnsureMainAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn create_thread(
        &self,
        user_id: i64,
        input: CreateAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn fork_thread(
        &self,
        user_id: i64,
        input: ForkAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn update_thread(
        &self,
        user_id: i64,
        input: UpdateAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn archive_thread(
        &self,
        user_id: i64,
        input: ArchiveAgentThreadInput,
    ) -> Result<AgentThreadView, a4n_error::AppError>;

    async fn list_threads(
        &self,
        user_id: i64,
        input: ListAgentThreadsInput,
    ) -> Result<Vec<AgentThreadView>, a4n_error::AppError>;

    async fn show_thread(
        &self,
        user_id: i64,
        thread_id: &str,
    ) -> Result<AgentThreadSnapshot, a4n_error::AppError>;

    async fn list_messages(
        &self,
        user_id: i64,
        input: ListAgentMessagesInput,
    ) -> Result<Vec<AgentMessageView>, a4n_error::AppError>;

    async fn show_run(
        &self,
        user_id: i64,
        run_id: &str,
    ) -> Result<AgentRunView, a4n_error::AppError>;

    async fn send_message(
        &self,
        user_id: i64,
        input: SendAgentMessageInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, a4n_error::AppError>;

    async fn control_run(
        &self,
        user_id: i64,
        input: ControlAgentRunInput,
    ) -> Result<AgentRunView, a4n_error::AppError>;

    async fn send_message_to_child(
        &self,
        user_id: i64,
        input: SendAgentMessageToChildInput,
        runtime_snapshot_required: bool,
        require_runtime_snapshot_for_new_run: bool,
    ) -> Result<AgentMessageAccepted, a4n_error::AppError>;

    async fn control_child_agent(
        &self,
        user_id: i64,
        input: ControlChildAgentInput,
    ) -> Result<AgentRunView, a4n_error::AppError>;

    async fn spawn_agent(
        &self,
        user_id: i64,
        input: SpawnAgentInput,
    ) -> Result<AgentDelegationView, a4n_error::AppError>;

    async fn events(
        &self,
        user_id: i64,
        input: AgentEventsInput,
    ) -> Result<Vec<AgentEvent>, a4n_error::AppError>;

    fn search_tools(
        &self,
        input: SearchAgentToolsInput,
    ) -> Result<AgentToolSearchResult, a4n_error::AppError>;

    fn describe_tool(
        &self,
        input: DescribeAgentToolInput,
    ) -> Result<AgentToolDescriptor, a4n_error::AppError>;

    async fn list_memories(
        &self,
        user_id: i64,
        scope_kind: Option<AgentMemoryScopeKind>,
        scope_id: Option<String>,
    ) -> Result<Vec<AgentMemoryView>, a4n_error::AppError>;

    async fn create_memory(
        &self,
        user_id: i64,
        input: AgentMemoryInput,
    ) -> Result<AgentMemoryView, a4n_error::AppError>;

    async fn retract_memory(
        &self,
        user_id: i64,
        input: RetractAgentMemoryInput,
    ) -> Result<AgentMemoryView, a4n_error::AppError>;

    async fn review_delegation(
        &self,
        user_id: i64,
        input: ReviewAgentDelegationInput,
        parent_run_id: Option<i64>,
    ) -> Result<AgentDelegationView, a4n_error::AppError>;

    async fn resolve_approval(
        &self,
        user_id: i64,
        input: ResolveAgentApprovalInput,
    ) -> Result<AgentApprovalView, a4n_error::AppError>;

    async fn verify_chapters_in_project(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), a4n_error::AppError>;
}

#[cfg(test)]
mod tests {
    use super::{
        CANVAS_AGENT_CAPABILITY, CONTENT_OPTIMIZATION_AGENT_CAPABILITY, DIRECTOR_AGENT_CAPABILITY,
        GENERAL_AGENT_CAPABILITY, LocateWritingTargetInput, MAIN_AGENT_CAPABILITY,
        agent_tools_for_capability, synthetic_agent_tool_descriptor,
    };
    use crate::command_manifest::command_descriptor;

    #[test]
    fn capability_catalogs_are_non_empty_and_every_tool_is_resolvable() {
        for capability in [
            MAIN_AGENT_CAPABILITY,
            GENERAL_AGENT_CAPABILITY,
            CONTENT_OPTIMIZATION_AGENT_CAPABILITY,
            CANVAS_AGENT_CAPABILITY,
            DIRECTOR_AGENT_CAPABILITY,
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
