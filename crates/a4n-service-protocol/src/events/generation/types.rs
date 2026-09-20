use a4n_generation_artifact_contract::GenerationArtifactRef;
use a4n_progress::{ProgressPhaseKind, ProgressSeverity};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de};

use crate::response::ServiceError;

use super::super::generation_error_codes::is_registered_generation_error_code;
use super::validation::{
    GenerationContractError, invalid, validate_generation_task_event,
    validate_generation_task_snapshot,
};
use super::{
    JS_SAFE_INTEGER_I64, JS_SAFE_INTEGER_U64, MAX_ERROR_MESSAGE_BYTES, MAX_FAILED_ITEM_KEYS,
    MAX_GENERATION_ARTIFACTS, MAX_GENERATION_STREAMS, MAX_IDENTIFIER_BYTES, MAX_KEY_BYTES,
    MAX_MESSAGE_BYTES, MAX_STREAM_ID_BYTES, MAX_TEXT_DELTA_BYTES,
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskEvent {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u16,
    #[schemars(length(equal = 36))]
    pub task_id: String,
    #[schemars(range(min = 1))]
    pub attempt: u32,
    #[schemars(range(min = 1, max = JS_SAFE_INTEGER_U64))]
    pub sequence: u64,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub emitted_at_ms: i64,
    pub event: GenerationEvent,
}

impl GenerationTaskEvent {
    pub fn validate(&self) -> Result<(), GenerationContractError> {
        validate_generation_task_event(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(
    tag = "kind",
    content = "data",
    rename_all = "snake_case",
    deny_unknown_fields
)]
pub enum GenerationEvent {
    Started(StartedEvent),
    StatusChanged(StatusChangedEvent),
    Progress(ProgressEvent),
    TextDelta(TextDeltaEvent),
    Usage(UsageEvent),
    Checkpoint(CheckpointEvent),
    Artifact(ArtifactEvent),
    CancelRequested(CancelRequestedEvent),
    Completed(CompletedEvent),
    Failed(FailedEvent),
    Cancelled(CancelledEvent),
    Interrupted(InterruptedEvent),
}

impl GenerationEvent {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed(_) | Self::Failed(_) | Self::Cancelled(_) | Self::Interrupted(_)
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationTaskStatus {
    Reserved,
    Running,
    Finalizing,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
    Interrupted,
}

impl GenerationTaskStatus {
    pub fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::Interrupted
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationDeliveryMode {
    Stream,
    StageOnly,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatchCommitPolicy {
    Atomic,
    BestEffort,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTargetRef {
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub kind: String,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub id: String,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub revision: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StartedEvent {
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub task_kind: String,
    pub delivery_mode: GenerationDeliveryMode,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub project_id: Option<i64>,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub root_project_id: Option<i64>,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub workflow_run_id: Option<i64>,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub generation_run_id: Option<i64>,
    pub target: Option<GenerationTargetRef>,
    pub batch_policy: Option<BatchCommitPolicy>,
    #[schemars(length(equal = 36))]
    pub root_task_id: String,
    #[schemars(length(equal = 36))]
    pub parent_task_id: Option<String>,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub item_key: Option<String>,
    #[schemars(length(equal = 36))]
    pub retry_of_task_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StatusChangedEvent {
    pub status: GenerationTaskStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CancellationReason {
    UserRequested,
    ParentCancelled,
    Superseded,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DraftDisposition {
    Discarded,
    #[default]
    RetainedUntilExpiry,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CancelRequestedEvent {
    pub reason: CancellationReason,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct FailedEvent {
    pub error: GenerationError,
    pub draft_disposition: DraftDisposition,
    pub batch_summary: Option<BatchSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CancelledEvent {
    pub reason: CancellationReason,
    pub draft_disposition: DraftDisposition,
    #[schemars(length(max = MAX_MESSAGE_BYTES))]
    pub summary: Option<String>,
    pub batch_summary: Option<BatchSummary>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum InterruptionReason {
    HostShutdown,
    HostRestartRecovery,
    SessionOwnerLost,
    DestructiveDrainTimeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct InterruptedEvent {
    pub reason: InterruptionReason,
    pub retryable: bool,
    pub draft_disposition: DraftDisposition,
    #[schemars(length(max = MAX_MESSAGE_BYTES))]
    pub summary: Option<String>,
    pub batch_summary: Option<BatchSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct BatchSummary {
    pub policy: BatchCommitPolicy,
    pub total: u32,
    pub succeeded: u32,
    pub failed: u32,
    pub cancelled: u32,
    pub skipped: u32,
    #[schemars(length(max = MAX_FAILED_ITEM_KEYS))]
    pub failed_item_keys: Vec<String>,
    pub failed_item_keys_truncated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TextDeltaEvent {
    #[schemars(length(min = 1, max = MAX_STREAM_ID_BYTES))]
    pub stream_id: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub offset: u64,
    #[schemars(length(min = 1, max = MAX_TEXT_DELTA_BYTES))]
    pub delta: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub chunk_index: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProgressEvent {
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub stage_code: String,
    pub phase_kind: ProgressPhaseKind,
    #[schemars(length(max = MAX_MESSAGE_BYTES))]
    pub message: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub current: Option<u64>,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub total: Option<u64>,
    #[schemars(length(max = MAX_KEY_BYTES))]
    pub unit_label: Option<String>,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub item_key: Option<String>,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub item_index: Option<u64>,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub item_total: Option<u64>,
    pub severity: ProgressSeverity,
    pub estimated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationUsage {
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub prompt_tokens: u64,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub completion_tokens: u64,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub total_tokens: u64,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub prompt_cache_hit_tokens: Option<u64>,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub prompt_cache_miss_tokens: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub prompt_cache_write_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct UsageEvent {
    pub usage: GenerationUsage,
    pub cumulative: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CheckpointEvent {
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub checkpoint_key: String,
    pub resumable: bool,
    #[schemars(length(max = MAX_MESSAGE_BYTES))]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactOperation {
    PreviewAvailable,
    Available,
    Updated,
    Removed,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ArtifactEvent {
    pub artifact: GenerationArtifactRef,
    pub operation: ArtifactOperation,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub replaces_artifact_id: Option<String>,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub item_key: Option<String>,
    pub item_index: Option<u32>,
    pub item_total: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CompletedEvent {
    pub result_ref: Option<GenerationResultRef>,
    #[schemars(length(max = MAX_GENERATION_STREAMS))]
    pub final_text_hashes: Vec<StreamTextHash>,
    pub final_usage: Option<GenerationUsage>,
    pub finish_reason: Option<GenerationFinishReason>,
    #[schemars(length(max = MAX_MESSAGE_BYTES))]
    pub summary: Option<String>,
    pub batch_summary: Option<BatchSummary>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GenerationFinishReason {
    Stop,
    ToolCallsComplete,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationResultRef {
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub kind: String,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub id: String,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct StreamTextHash {
    #[schemars(length(min = 1, max = MAX_STREAM_ID_BYTES))]
    pub stream_id: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub utf8_bytes: u64,
    #[schemars(length(equal = 64))]
    pub sha256: String,
}

#[derive(Debug, Clone, Serialize, JsonSchema, PartialEq, Eq)]
#[serde(transparent)]
pub struct GenerationErrorCode(
    #[schemars(
        length(min = 1, max = MAX_IDENTIFIER_BYTES),
        regex(pattern = "^[a-z][a-z0-9_]{0,127}$")
    )]
    String,
);

impl GenerationErrorCode {
    pub fn registered(value: impl AsRef<str>) -> Result<Self, GenerationContractError> {
        let value = value.as_ref();
        if !is_registered_generation_error_code(value) {
            return Err(invalid(format!("未注册的生成错误码: {value}")));
        }
        Ok(Self(value.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl<'de> Deserialize<'de> for GenerationErrorCode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::registered(value).map_err(de::Error::custom)
    }
}

impl std::fmt::Display for GenerationErrorCode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationError {
    pub code: GenerationErrorCode,
    #[schemars(length(max = MAX_ERROR_MESSAGE_BYTES))]
    pub message: String,
    pub retryable: bool,
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub stage_code: String,
    #[schemars(length(max = MAX_KEY_BYTES))]
    pub trace_id: Option<String>,
    pub source_error: Option<Box<ServiceError>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationTaskSnapshot {
    #[schemars(range(min = 1, max = 1))]
    pub schema_version: u16,
    #[schemars(length(equal = 36))]
    pub task_id: String,
    #[schemars(range(min = 1))]
    pub attempt: u32,
    #[schemars(range(min = 1, max = JS_SAFE_INTEGER_U64))]
    pub covers_through_sequence: u64,
    pub status: GenerationTaskStatus,
    pub started: StartedEvent,
    pub cancel_reason: Option<CancellationReason>,
    pub progress: Option<ProgressEvent>,
    #[schemars(length(max = MAX_GENERATION_STREAMS))]
    pub streams: Vec<TextStreamSnapshot>,
    #[schemars(length(max = MAX_GENERATION_ARTIFACTS))]
    pub artifacts: Vec<GenerationArtifactRef>,
    pub usage: Option<GenerationUsage>,
    pub latest_checkpoint: Option<CheckpointEvent>,
    pub terminal_event: Option<GenerationTaskEvent>,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub updated_at_ms: i64,
}

impl GenerationTaskSnapshot {
    pub fn validate(&self) -> Result<(), GenerationContractError> {
        validate_generation_task_snapshot(self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct TextStreamSnapshot {
    #[schemars(length(min = 1, max = MAX_STREAM_ID_BYTES))]
    pub stream_id: String,
    pub text: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub next_offset: u64,
    #[schemars(length(equal = 64))]
    pub sha256: String,
}
