use super::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
    /// Derived from `chapter_id` by the Agent backend; never use it for authorization.
    #[serde(default)]
    pub chapter_number: Option<i32>,
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
    /// Estimated tokens in the latest fitted Agent model input.
    #[serde(default)]
    pub context_input_tokens: Option<u64>,
    /// Safe input capacity used by the latest fitted Agent model request.
    #[serde(default)]
    pub context_capacity_tokens: Option<u64>,
    #[serde(default)]
    pub cost_micros: Option<u64>,
    #[serde(default)]
    pub error: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub started_at: Option<String>,
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

/// Skill metadata is intentionally small enough to keep in every Agent model
/// request. The body is returned only by `load_agent_skill`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub version: String,
    pub source: String,
    pub content_hash: String,
    pub capabilities: Vec<String>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillContent {
    pub skill: AgentSkillDescriptor,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillSearchResult {
    pub skills: Vec<AgentSkillDescriptor>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillProfileView {
    pub capability: String,
    pub skills: Vec<AgentSkillDescriptor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AgentSkillSettingsView {
    pub profiles: Vec<AgentSkillProfileView>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct SearchAgentSkillsInput {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct LoadAgentSkillInput {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UpdateAgentSkillConfigurationInput {
    pub capability: String,
    pub enabled_skill_ids: Vec<String>,
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
