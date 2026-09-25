use super::*;

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
    #[serde(default)]
    pub chapter_number: Option<i32>,
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
        plan: crate::chapter_visual_plan::ChapterVisualPlanV3,
    },
    SaveDraft {
        state: AgentEditorState,
        plan: crate::chapter_visual_plan::ChapterVisualPlanV3,
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
