use a4n_service_protocol::{
    GenerationErrorCode, GenerationResultRef, GenerationTaskEvent, GenerationTaskStatus,
    JS_SAFE_INTEGER_I64, JS_SAFE_INTEGER_U64, StartedEvent,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationSubscriptionHandle {
    #[schemars(length(min = 1, max = 128))]
    pub subscription_id: String,
    #[schemars(length(equal = 36))]
    pub task_id: String,
    #[schemars(range(min = 1))]
    pub attempt: u32,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_U64))]
    pub subscribed_after_sequence: Option<u64>,
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
pub struct UnsubscribeGenerationTaskInput {
    #[schemars(length(min = 1, max = 128))]
    pub subscription_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct HostErrorResponse {
    pub code: GenerationErrorCode,
    #[schemars(length(max = 4096))]
    pub message: String,
    pub retryable: bool,
    #[schemars(length(max = 128))]
    pub trace_id: Option<String>,
}
