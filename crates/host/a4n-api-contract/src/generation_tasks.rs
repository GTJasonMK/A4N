use a4n_service_protocol::{
    GenerationErrorCode, GenerationResultRef, GenerationTaskEvent, GenerationTaskStatus,
    JS_SAFE_INTEGER_I64, StartedEvent,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const GENERATION_START_ROUTES: &[(&str, &str)] = &[
    ("start_blueprint_generation", "/generation-tasks/blueprint"),
    (
        "start_blueprint_refinement",
        "/generation-tasks/blueprint-refinement",
    ),
    ("start_chapter_generation", "/generation-tasks/chapter"),
    ("start_chapter_shooting_script_generation", "/generation-tasks/shooting-script"),
    ("start_image_generation", "/generation-tasks/image"),
    (
        "start_visual_plan_prompt_generation",
        "/generation-tasks/director-prompt",
    ),
    (
        "start_inspiration_generation",
        "/generation-tasks/inspiration",
    ),
    (
        "start_optimize_analysis_generation",
        "/generation-tasks/optimize-analysis",
    ),
    (
        "start_optimize_chat_generation",
        "/generation-tasks/optimize-chat",
    ),
    (
        "start_optimize_patch_generation",
        "/generation-tasks/optimize-patch",
    ),
    ("start_outline_generation", "/generation-tasks/outline"),
    ("start_portrait_generation", "/generation-tasks/portrait"),
    ("start_video_generation", "/generation-tasks/video"),
];

pub fn generation_start_path(command: &str) -> Option<&'static str> {
    GENERATION_START_ROUTES
        .iter()
        .find_map(|(candidate, path)| (*candidate == command).then_some(*path))
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationStartRequestMeta {
    #[schemars(length(min = 1, max = 128))]
    pub start_request_id: String,
    #[schemars(length(equal = 36))]
    pub retry_of_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskHandle {
    #[schemars(range(min = 1, max = 1))]
    pub event_schema_version: u16,
    #[schemars(length(equal = 36))]
    pub task_id: String,
    #[schemars(range(min = 1))]
    pub attempt: u32,
    pub status: GenerationTaskStatus,
}

/// Stable result returned when an application owner reserves a generation task.
///
/// Queue/session internals remain private to the owner; Runtime only needs the
/// handle and whether it must dispatch a worker for this request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskStart {
    pub handle: GenerationTaskHandle,
    pub should_run: bool,
}

/// Stable failure returned by a generation-specific start port.
///
/// The owner maps persistence, queue, and authorization details to a public
/// code/message pair instead of exposing its internal error graph.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskStartError {
    #[schemars(length(min = 1, max = 128))]
    code: String,
    #[schemars(length(min = 1, max = 4096))]
    message: String,
    pub retryable: bool,
}

impl GenerationTaskStartError {
    pub fn new(code: impl Into<String>, message: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            retryable,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }

    pub fn message(&self) -> &str {
        &self.message
    }
}

impl std::fmt::Display for GenerationTaskStartError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for GenerationTaskStartError {}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationSubscriptionHandle {
    #[schemars(length(min = 1, max = 128))]
    pub subscription_id: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationSnapshotState {
    Available,
    Expired,
    Corrupt,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskSummary {
    pub handle: GenerationTaskHandle,
    pub started: StartedEvent,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub updated_at_ms: i64,
    pub result_ref: Option<GenerationResultRef>,
    pub snapshot_state: GenerationSnapshotState,
    pub terminal_event: Option<GenerationTaskEvent>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskHistoryQuery {
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub project_id: Option<i64>,
    #[schemars(range(min = 1, max = 100))]
    pub limit: u32,
    #[schemars(length(max = 512))]
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskHistoryPage {
    pub items: Vec<GenerationTaskSummary>,
    #[schemars(length(max = 512))]
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CancelGenerationTaskInput {
    #[schemars(length(equal = 36))]
    pub task_id: String,
    #[schemars(range(min = 1))]
    pub attempt: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostErrorResponse {
    pub code: GenerationErrorCode,
    #[schemars(length(max = 4096))]
    pub message: String,
    pub retryable: bool,
    #[serde(deserialize_with = "crate::serde_update::deserialize_required_nullable_field")]
    #[schemars(
        required,
        schema_with = "crate::serde_update::nullable_trace_id_schema"
    )]
    pub trace_id: Option<String>,
    pub details: serde_json::Map<String, serde_json::Value>,
}
