use std::collections::{BTreeMap, BTreeSet};

use a4n_domain::progress::{ProgressPhaseKind, ProgressSeverity};
use schemars::JsonSchema;
use serde::{Deserialize, Deserializer, Serialize, de};
use sha2::{Digest, Sha256};
use uuid::{Uuid, Version};

use crate::response::ServiceError;

use super::generation_error_codes::is_registered_generation_error_code;

pub const GENERATION_EVENT_SCHEMA_VERSION: u16 = 1;
pub const JS_SAFE_INTEGER_U64: u64 = 9_007_199_254_740_991;
pub const JS_SAFE_INTEGER_I64: i64 = 9_007_199_254_740_991;
pub const MAX_GENERATION_EVENT_BYTES: usize = 64 * 1024;
pub const MAX_TEXT_DELTA_BYTES: usize = 16 * 1024;
pub const MAX_GENERATION_SNAPSHOT_BYTES: usize = 4 * 1024 * 1024;
const MAX_GENERATION_STREAMS: usize = 64;
const MAX_GENERATION_ARTIFACTS: usize = 4_096;
pub const DEFAULT_GENERATION_ACTIVE_SESSION_MAX: usize = 64;

const MAX_IDENTIFIER_BYTES: usize = 128;
const MAX_STREAM_ID_BYTES: usize = 64;
const MAX_KEY_BYTES: usize = 256;
const MAX_MESSAGE_BYTES: usize = 2 * 1024;
const MAX_ERROR_MESSAGE_BYTES: usize = 4 * 1024;
const MAX_FAILED_ITEM_KEYS: usize = 256;

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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct GenerationArtifactRef {
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub artifact_id: String,
    #[schemars(length(min = 1, max = MAX_IDENTIFIER_BYTES))]
    pub artifact_kind: String,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub content_type: String,
    #[schemars(length(equal = 64))]
    pub sha256: String,
    #[schemars(range(max = JS_SAFE_INTEGER_U64))]
    pub byte_size: u64,
    #[schemars(length(min = 1, max = MAX_KEY_BYTES))]
    pub revision: String,
    pub temporary: bool,
    #[schemars(range(min = 0, max = JS_SAFE_INTEGER_I64))]
    pub expires_at_ms: Option<i64>,
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
    pub source_error: Option<ServiceError>,
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

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("生成事件合同无效: {0}")]
pub struct GenerationContractError(String);

fn validate_generation_task_event(
    event: &GenerationTaskEvent,
) -> Result<(), GenerationContractError> {
    if event.schema_version != GENERATION_EVENT_SCHEMA_VERSION {
        return Err(invalid(format!(
            "schema_version 必须为 {GENERATION_EVENT_SCHEMA_VERSION}"
        )));
    }
    validate_uuid_v4("task_id", &event.task_id)?;
    if event.attempt == 0 {
        return Err(invalid("attempt 必须大于 0"));
    }
    validate_safe_u64("sequence", event.sequence, false)?;
    validate_safe_i64("emitted_at_ms", event.emitted_at_ms)?;

    match &event.event {
        GenerationEvent::Started(started) => {
            if event.sequence != 1 {
                return Err(invalid("started 固定使用 sequence=1"));
            }
            validate_started_event(started, &event.task_id)?;
        }
        GenerationEvent::StatusChanged(status) => {
            require_after_started(event.sequence)?;
            if !matches!(
                status.status,
                GenerationTaskStatus::Running | GenerationTaskStatus::Finalizing
            ) {
                return Err(invalid("status_changed 只允许 running 或 finalizing"));
            }
        }
        GenerationEvent::Progress(progress) => {
            require_after_started(event.sequence)?;
            validate_progress_event(progress)?;
        }
        GenerationEvent::TextDelta(delta) => {
            require_after_started(event.sequence)?;
            validate_text_delta_event(delta)?;
        }
        GenerationEvent::Usage(usage) => {
            require_after_started(event.sequence)?;
            if !usage.cumulative {
                return Err(invalid("V1 usage 必须是累计值"));
            }
            validate_usage(&usage.usage)?;
        }
        GenerationEvent::Checkpoint(checkpoint) => {
            require_after_started(event.sequence)?;
            validate_checkpoint_event(checkpoint)?;
        }
        GenerationEvent::Artifact(artifact) => {
            require_after_started(event.sequence)?;
            validate_artifact_event(artifact)?;
        }
        GenerationEvent::CancelRequested(_) => require_after_started(event.sequence)?,
        GenerationEvent::Completed(completed) => {
            require_after_started(event.sequence)?;
            validate_completed_event(completed)?;
        }
        GenerationEvent::Failed(failed) => {
            require_after_started(event.sequence)?;
            validate_generation_error(&failed.error)?;
            validate_optional_batch_summary(failed.batch_summary.as_ref())?;
        }
        GenerationEvent::Cancelled(cancelled) => {
            require_after_started(event.sequence)?;
            validate_optional_text(
                "cancelled.summary",
                cancelled.summary.as_deref(),
                MAX_MESSAGE_BYTES,
            )?;
            validate_optional_batch_summary(cancelled.batch_summary.as_ref())?;
        }
        GenerationEvent::Interrupted(interrupted) => {
            require_after_started(event.sequence)?;
            validate_optional_text(
                "interrupted.summary",
                interrupted.summary.as_deref(),
                MAX_MESSAGE_BYTES,
            )?;
            validate_optional_batch_summary(interrupted.batch_summary.as_ref())?;
        }
    }

    let bytes =
        serde_json::to_vec(event).map_err(|error| invalid(format!("事件序列化失败: {error}")))?;
    if bytes.len() > MAX_GENERATION_EVENT_BYTES {
        return Err(invalid(format!(
            "单事件超过 {} 字节上限",
            MAX_GENERATION_EVENT_BYTES
        )));
    }
    Ok(())
}

fn validate_generation_task_snapshot(
    snapshot: &GenerationTaskSnapshot,
) -> Result<(), GenerationContractError> {
    if snapshot.schema_version != GENERATION_EVENT_SCHEMA_VERSION {
        return Err(invalid(format!(
            "snapshot schema_version 必须为 {GENERATION_EVENT_SCHEMA_VERSION}"
        )));
    }
    validate_uuid_v4("snapshot.task_id", &snapshot.task_id)?;
    if snapshot.attempt == 0 {
        return Err(invalid("snapshot.attempt 必须大于 0"));
    }
    validate_safe_u64(
        "snapshot.covers_through_sequence",
        snapshot.covers_through_sequence,
        false,
    )?;
    validate_safe_i64("snapshot.updated_at_ms", snapshot.updated_at_ms)?;
    validate_started_event(&snapshot.started, &snapshot.task_id)?;

    if snapshot.status == GenerationTaskStatus::Cancelling {
        if snapshot.cancel_reason.is_none() {
            return Err(invalid("cancelling snapshot 必须包含 cancel_reason"));
        }
    } else if snapshot.cancel_reason.is_some() {
        return Err(invalid("仅 cancelling snapshot 可包含 cancel_reason"));
    }

    if let Some(progress) = &snapshot.progress {
        validate_progress_event(progress)?;
    }
    if snapshot.streams.len() > MAX_GENERATION_STREAMS {
        return Err(invalid(format!(
            "snapshot text stream 超过 {MAX_GENERATION_STREAMS} 个"
        )));
    }
    let mut streams = BTreeMap::new();
    for stream in &snapshot.streams {
        validate_stream_id(&stream.stream_id)?;
        validate_safe_u64("stream.next_offset", stream.next_offset, true)?;
        let expected_offset = stream.text.len() as u64;
        if stream.next_offset != expected_offset {
            return Err(invalid(format!(
                "stream {} next_offset={}，实际 UTF-8 字节数={expected_offset}",
                stream.stream_id, stream.next_offset
            )));
        }
        validate_sha256("stream.sha256", &stream.sha256)?;
        let actual_sha = format!("{:x}", Sha256::digest(stream.text.as_bytes()));
        if stream.sha256 != actual_sha {
            return Err(invalid(format!(
                "stream {} sha256 不匹配",
                stream.stream_id
            )));
        }
        if streams.insert(stream.stream_id.as_str(), stream).is_some() {
            return Err(invalid(format!(
                "snapshot stream_id 重复: {}",
                stream.stream_id
            )));
        }
    }

    if snapshot.artifacts.len() > MAX_GENERATION_ARTIFACTS {
        return Err(invalid(format!(
            "snapshot artifact 超过 {MAX_GENERATION_ARTIFACTS} 个"
        )));
    }
    let mut artifact_ids = BTreeSet::new();
    for artifact in &snapshot.artifacts {
        validate_generation_artifact_ref(artifact)?;
        if !artifact_ids.insert(artifact.artifact_id.as_str()) {
            return Err(invalid(format!(
                "snapshot artifact_id 重复: {}",
                artifact.artifact_id
            )));
        }
    }
    if let Some(usage) = &snapshot.usage {
        validate_usage(usage)?;
    }
    if let Some(checkpoint) = &snapshot.latest_checkpoint {
        validate_checkpoint_event(checkpoint)?;
    }

    match (&snapshot.status, &snapshot.terminal_event) {
        (status, None) if !status.is_terminal() => {}
        (status, Some(_)) if !status.is_terminal() => {
            return Err(invalid(format!(
                "active snapshot status={status:?} 不得包含 terminal_event"
            )));
        }
        (status, None) => {
            return Err(invalid(format!(
                "terminal snapshot status={status:?} 必须包含 terminal_event"
            )));
        }
        (status, Some(terminal)) => {
            terminal.validate()?;
            if terminal.task_id != snapshot.task_id
                || terminal.attempt != snapshot.attempt
                || terminal.sequence != snapshot.covers_through_sequence
            {
                return Err(invalid(
                    "terminal_event 的 task_id/attempt/sequence 与 snapshot 不一致",
                ));
            }
            validate_terminal_status(*status, &terminal.event)?;
            validate_terminal_batch_contract(&snapshot.started, &terminal.event)?;
            if let GenerationEvent::Completed(completed) = &terminal.event {
                validate_completed_against_snapshot(completed, &streams, snapshot.usage.as_ref())?;
            }
        }
    }

    let bytes = serde_json::to_vec(snapshot)
        .map_err(|error| invalid(format!("snapshot 序列化失败: {error}")))?;
    if bytes.len() > MAX_GENERATION_SNAPSHOT_BYTES {
        return Err(invalid(format!(
            "snapshot 超过 {} 字节上限",
            MAX_GENERATION_SNAPSHOT_BYTES
        )));
    }
    Ok(())
}

fn validate_started_event(
    started: &StartedEvent,
    task_id: &str,
) -> Result<(), GenerationContractError> {
    validate_identifier(
        "started.task_kind",
        &started.task_kind,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_optional_safe_i64("started.project_id", started.project_id)?;
    validate_optional_safe_i64("started.root_project_id", started.root_project_id)?;
    validate_optional_safe_i64("started.workflow_run_id", started.workflow_run_id)?;
    validate_optional_safe_i64("started.generation_run_id", started.generation_run_id)?;
    if let Some(target) = &started.target {
        validate_identifier("target.kind", &target.kind, MAX_IDENTIFIER_BYTES)?;
        validate_bounded_nonempty("target.id", &target.id, MAX_KEY_BYTES)?;
        if let Some(revision) = &target.revision {
            validate_bounded_nonempty("target.revision", revision, MAX_KEY_BYTES)?;
        }
    }
    validate_uuid_v4("started.root_task_id", &started.root_task_id)?;
    if let Some(parent_task_id) = &started.parent_task_id {
        validate_uuid_v4("started.parent_task_id", parent_task_id)?;
        if parent_task_id == task_id {
            return Err(invalid("child parent_task_id 不能等于自身 task_id"));
        }
        if started.root_task_id == task_id {
            return Err(invalid("child root_task_id 不能等于自身 task_id"));
        }
        validate_bounded_nonempty(
            "started.item_key",
            started.item_key.as_deref().unwrap_or_default(),
            MAX_KEY_BYTES,
        )?;
    } else {
        if started.root_task_id != task_id {
            return Err(invalid("根任务 root_task_id 必须等于 envelope task_id"));
        }
        if started.item_key.is_some() {
            return Err(invalid("根任务不得包含 item_key"));
        }
    }
    if let Some(retry_of) = &started.retry_of_task_id {
        validate_uuid_v4("started.retry_of_task_id", retry_of)?;
        if retry_of == task_id {
            return Err(invalid("retry_of_task_id 不能等于当前 task_id"));
        }
    }
    Ok(())
}

fn validate_progress_event(progress: &ProgressEvent) -> Result<(), GenerationContractError> {
    validate_identifier(
        "progress.stage_code",
        &progress.stage_code,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_text("progress.message", &progress.message, MAX_MESSAGE_BYTES)?;
    validate_optional_text(
        "progress.unit_label",
        progress.unit_label.as_deref(),
        MAX_KEY_BYTES,
    )?;
    if let Some(item_key) = &progress.item_key {
        validate_bounded_nonempty("progress.item_key", item_key, MAX_KEY_BYTES)?;
    }
    match (progress.current, progress.total) {
        (None, None) => {}
        (Some(current), Some(total)) => {
            validate_safe_u64("progress.current", current, true)?;
            validate_safe_u64("progress.total", total, false)?;
            if current > total {
                return Err(invalid("progress.current 不能大于 total"));
            }
        }
        _ => return Err(invalid("progress.current/total 必须同时有值或同时为空")),
    }
    match (progress.item_index, progress.item_total) {
        (None, None) => {}
        (Some(index), Some(total)) => {
            validate_safe_u64("progress.item_index", index, true)?;
            validate_safe_u64("progress.item_total", total, false)?;
            if index >= total {
                return Err(invalid("progress.item_index 必须小于 item_total"));
            }
        }
        _ => return Err(invalid("progress.item_index/item_total 必须成对出现")),
    }
    Ok(())
}

fn validate_text_delta_event(delta: &TextDeltaEvent) -> Result<(), GenerationContractError> {
    validate_stream_id(&delta.stream_id)?;
    validate_safe_u64("text_delta.offset", delta.offset, true)?;
    validate_safe_u64("text_delta.chunk_index", delta.chunk_index, true)?;
    if delta.delta.is_empty() {
        return Err(invalid("text_delta.delta 不能为空"));
    }
    if delta.delta.len() > MAX_TEXT_DELTA_BYTES {
        return Err(invalid(format!(
            "text_delta.delta 超过 {MAX_TEXT_DELTA_BYTES} 字节"
        )));
    }
    Ok(())
}

fn validate_usage(usage: &GenerationUsage) -> Result<(), GenerationContractError> {
    validate_safe_u64("usage.prompt_tokens", usage.prompt_tokens, true)?;
    validate_safe_u64("usage.completion_tokens", usage.completion_tokens, true)?;
    validate_safe_u64("usage.total_tokens", usage.total_tokens, true)?;
    let total = usage
        .prompt_tokens
        .checked_add(usage.completion_tokens)
        .ok_or_else(|| invalid("usage token 计数溢出"))?;
    if usage.total_tokens != total {
        return Err(invalid(
            "usage.total_tokens 必须等于 prompt_tokens + completion_tokens",
        ));
    }
    let cache_hit = usage.prompt_cache_hit_tokens.unwrap_or(0);
    let cache_miss = usage.prompt_cache_miss_tokens.unwrap_or(0);
    validate_safe_u64("usage.prompt_cache_hit_tokens", cache_hit, true)?;
    validate_safe_u64("usage.prompt_cache_miss_tokens", cache_miss, true)?;
    if cache_hit
        .checked_add(cache_miss)
        .is_none_or(|value| value > usage.prompt_tokens)
    {
        return Err(invalid("prompt cache hit/miss 计数不能超过 prompt_tokens"));
    }
    Ok(())
}

fn validate_checkpoint_event(checkpoint: &CheckpointEvent) -> Result<(), GenerationContractError> {
    validate_bounded_nonempty(
        "checkpoint.checkpoint_key",
        &checkpoint.checkpoint_key,
        MAX_KEY_BYTES,
    )?;
    validate_optional_text(
        "checkpoint.summary",
        checkpoint.summary.as_deref(),
        MAX_MESSAGE_BYTES,
    )
}

fn validate_artifact_event(artifact: &ArtifactEvent) -> Result<(), GenerationContractError> {
    validate_generation_artifact_ref(&artifact.artifact)?;
    match artifact.operation {
        ArtifactOperation::PreviewAvailable => {
            if !artifact.artifact.temporary
                || artifact.artifact.expires_at_ms.is_none()
                || artifact.replaces_artifact_id.is_some()
            {
                return Err(invalid(
                    "preview_available 必须 temporary=true、包含 expires_at_ms 且不替换其他资产",
                ));
            }
        }
        ArtifactOperation::Available => {
            if artifact.artifact.temporary || artifact.replaces_artifact_id.is_some() {
                return Err(invalid(
                    "available 必须 temporary=false 且 replaces_artifact_id 为空",
                ));
            }
        }
        ArtifactOperation::Updated => {
            let Some(replaced) = artifact.replaces_artifact_id.as_deref() else {
                return Err(invalid("updated 必须包含 replaces_artifact_id"));
            };
            if artifact.artifact.temporary {
                return Err(invalid("updated 必须指向正式非临时资产"));
            }
            validate_bounded_nonempty("artifact.replaces_artifact_id", replaced, MAX_KEY_BYTES)?;
            if replaced == artifact.artifact.artifact_id {
                return Err(invalid("资产不能替换自身"));
            }
        }
        ArtifactOperation::Removed => {
            if artifact.replaces_artifact_id.is_some() {
                return Err(invalid("removed 不得包含 replaces_artifact_id"));
            }
        }
    }
    if let Some(item_key) = &artifact.item_key {
        validate_bounded_nonempty("artifact.item_key", item_key, MAX_KEY_BYTES)?;
    }
    match (artifact.item_index, artifact.item_total) {
        (None, None) => {}
        (Some(index), Some(total)) if total > 0 && index < total => {}
        (Some(_), Some(_)) => {
            return Err(invalid("artifact.item_index 必须小于非零 item_total"));
        }
        _ => return Err(invalid("artifact.item_index/item_total 必须成对出现")),
    }
    Ok(())
}

fn validate_generation_artifact_ref(
    artifact: &GenerationArtifactRef,
) -> Result<(), GenerationContractError> {
    validate_bounded_nonempty("artifact.artifact_id", &artifact.artifact_id, MAX_KEY_BYTES)?;
    validate_identifier(
        "artifact.artifact_kind",
        &artifact.artifact_kind,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_bounded_nonempty(
        "artifact.content_type",
        &artifact.content_type,
        MAX_KEY_BYTES,
    )?;
    validate_sha256("artifact.sha256", &artifact.sha256)?;
    validate_safe_u64("artifact.byte_size", artifact.byte_size, true)?;
    validate_bounded_nonempty("artifact.revision", &artifact.revision, MAX_KEY_BYTES)?;
    if let Some(expires_at_ms) = artifact.expires_at_ms {
        validate_safe_i64("artifact.expires_at_ms", expires_at_ms)?;
    }
    if artifact.temporary && artifact.expires_at_ms.is_none() {
        return Err(invalid("temporary artifact 必须包含 expires_at_ms"));
    }
    if !artifact.temporary && artifact.expires_at_ms.is_some() {
        return Err(invalid("正式 artifact 不得包含 expires_at_ms"));
    }
    Ok(())
}

fn validate_completed_event(completed: &CompletedEvent) -> Result<(), GenerationContractError> {
    if let Some(result) = &completed.result_ref {
        validate_identifier(
            "completed.result_ref.kind",
            &result.kind,
            MAX_IDENTIFIER_BYTES,
        )?;
        validate_bounded_nonempty("completed.result_ref.id", &result.id, MAX_KEY_BYTES)?;
        validate_bounded_nonempty(
            "completed.result_ref.revision",
            &result.revision,
            MAX_KEY_BYTES,
        )?;
    }
    if completed.final_text_hashes.len() > MAX_GENERATION_STREAMS {
        return Err(invalid(format!(
            "completed.final_text_hashes 超过 {MAX_GENERATION_STREAMS} 项"
        )));
    }
    let mut stream_ids = BTreeSet::new();
    for hash in &completed.final_text_hashes {
        validate_stream_id(&hash.stream_id)?;
        validate_safe_u64(
            "completed.final_text_hash.utf8_bytes",
            hash.utf8_bytes,
            true,
        )?;
        validate_sha256("completed.final_text_hash.sha256", &hash.sha256)?;
        if !stream_ids.insert(hash.stream_id.as_str()) {
            return Err(invalid(format!(
                "completed stream_id 重复: {}",
                hash.stream_id
            )));
        }
    }
    if let Some(usage) = &completed.final_usage {
        validate_usage(usage)?;
    }
    validate_optional_text(
        "completed.summary",
        completed.summary.as_deref(),
        MAX_MESSAGE_BYTES,
    )?;
    validate_optional_batch_summary(completed.batch_summary.as_ref())
}

fn validate_generation_error(error: &GenerationError) -> Result<(), GenerationContractError> {
    if !is_registered_generation_error_code(error.code.as_str()) {
        return Err(invalid(format!("未注册的生成错误码: {}", error.code)));
    }
    validate_text(
        "generation_error.message",
        &error.message,
        MAX_ERROR_MESSAGE_BYTES,
    )?;
    validate_identifier(
        "generation_error.stage_code",
        &error.stage_code,
        MAX_IDENTIFIER_BYTES,
    )?;
    validate_optional_text(
        "generation_error.trace_id",
        error.trace_id.as_deref(),
        MAX_KEY_BYTES,
    )
}

fn validate_optional_batch_summary(
    summary: Option<&BatchSummary>,
) -> Result<(), GenerationContractError> {
    if let Some(summary) = summary {
        validate_batch_summary(summary)?;
    }
    Ok(())
}

fn validate_batch_summary(summary: &BatchSummary) -> Result<(), GenerationContractError> {
    if summary.total == 0 {
        return Err(invalid("batch_summary.total 必须大于 0"));
    }
    let counted = summary
        .succeeded
        .checked_add(summary.failed)
        .and_then(|value| value.checked_add(summary.cancelled))
        .and_then(|value| value.checked_add(summary.skipped))
        .ok_or_else(|| invalid("batch_summary 计数溢出"))?;
    if counted != summary.total {
        return Err(invalid("batch_summary 各计数之和必须等于 total"));
    }
    if summary.failed_item_keys.len() > MAX_FAILED_ITEM_KEYS {
        return Err(invalid(format!(
            "failed_item_keys 最多 {MAX_FAILED_ITEM_KEYS} 项"
        )));
    }
    let mut previous: Option<&str> = None;
    for key in &summary.failed_item_keys {
        validate_bounded_nonempty("batch_summary.failed_item_key", key, MAX_KEY_BYTES)?;
        if previous.is_some_and(|value| value >= key.as_str()) {
            return Err(invalid(
                "failed_item_keys 必须去重并按 UTF-8 字节序严格排序",
            ));
        }
        previous = Some(key);
    }
    if summary.failed_item_keys.len() > summary.failed as usize {
        return Err(invalid("failed_item_keys 数量不能大于 failed"));
    }
    if summary.failed_item_keys_truncated {
        if summary.failed_item_keys.len() >= summary.failed as usize {
            return Err(invalid(
                "truncated=true 时 failed_item_keys 必须是不完整子集",
            ));
        }
    } else if summary.failed_item_keys.len() != summary.failed as usize {
        return Err(invalid(
            "truncated=false 时 failed_item_keys 数量必须等于 failed",
        ));
    }
    Ok(())
}

fn validate_terminal_status(
    status: GenerationTaskStatus,
    event: &GenerationEvent,
) -> Result<(), GenerationContractError> {
    let matches = matches!(
        (status, event),
        (
            GenerationTaskStatus::Completed,
            GenerationEvent::Completed(_)
        ) | (GenerationTaskStatus::Failed, GenerationEvent::Failed(_))
            | (
                GenerationTaskStatus::Cancelled,
                GenerationEvent::Cancelled(_)
            )
            | (
                GenerationTaskStatus::Interrupted,
                GenerationEvent::Interrupted(_)
            )
    );
    if matches {
        Ok(())
    } else {
        Err(invalid(format!(
            "snapshot status={status:?} 与 terminal event 不一致"
        )))
    }
}

fn validate_terminal_batch_contract(
    started: &StartedEvent,
    terminal: &GenerationEvent,
) -> Result<(), GenerationContractError> {
    let summary = match terminal {
        GenerationEvent::Completed(event) => event.batch_summary.as_ref(),
        GenerationEvent::Failed(event) => event.batch_summary.as_ref(),
        GenerationEvent::Cancelled(event) => event.batch_summary.as_ref(),
        GenerationEvent::Interrupted(event) => event.batch_summary.as_ref(),
        _ => return Ok(()),
    };
    match (started.batch_policy, summary) {
        (None, None) => Ok(()),
        (Some(policy), Some(summary)) if summary.policy == policy => {
            if matches!(terminal, GenerationEvent::Completed(_)) {
                match policy {
                    BatchCommitPolicy::Atomic
                        if summary.succeeded != summary.total || summary.failed != 0 =>
                    {
                        return Err(invalid("atomic batch 只有全部成功时才能 completed"));
                    }
                    BatchCommitPolicy::BestEffort if summary.succeeded == 0 => {
                        return Err(invalid("best_effort batch completed 至少需要一个成功子项"));
                    }
                    _ => {}
                }
            }
            Ok(())
        }
        (Some(_), None) => Err(invalid("批量任务 terminal 必须包含 batch_summary")),
        (None, Some(_)) => Err(invalid("非批量任务 terminal 不得包含 batch_summary")),
        (Some(_), Some(_)) => Err(invalid(
            "terminal batch_summary.policy 与 started.batch_policy 不一致",
        )),
    }
}

fn validate_completed_against_snapshot(
    completed: &CompletedEvent,
    streams: &BTreeMap<&str, &TextStreamSnapshot>,
    snapshot_usage: Option<&GenerationUsage>,
) -> Result<(), GenerationContractError> {
    if completed.final_text_hashes.len() != streams.len() {
        return Err(invalid(
            "completed.final_text_hashes 必须覆盖 snapshot 的全部 stream",
        ));
    }
    for hash in &completed.final_text_hashes {
        let Some(stream) = streams.get(hash.stream_id.as_str()) else {
            return Err(invalid(format!(
                "completed 引用了 snapshot 中不存在的 stream: {}",
                hash.stream_id
            )));
        };
        if hash.utf8_bytes != stream.next_offset || hash.sha256 != stream.sha256 {
            return Err(invalid(format!(
                "completed stream {} 的字节数或 sha256 与 snapshot 不一致",
                hash.stream_id
            )));
        }
    }
    if completed.final_usage.as_ref() != snapshot_usage {
        return Err(invalid(
            "completed.final_usage 必须与 snapshot 累计 usage 一致",
        ));
    }
    Ok(())
}

fn require_after_started(sequence: u64) -> Result<(), GenerationContractError> {
    if sequence <= 1 {
        Err(invalid("started 之外的事件 sequence 必须大于 1"))
    } else {
        Ok(())
    }
}

fn validate_uuid_v4(field: &str, value: &str) -> Result<(), GenerationContractError> {
    let parsed =
        Uuid::parse_str(value).map_err(|_| invalid(format!("{field} 必须是 UUID v4 文本")))?;
    if parsed.get_version() != Some(Version::Random) || parsed.hyphenated().to_string() != value {
        return Err(invalid(format!("{field} 必须是规范小写 UUID v4 文本")));
    }
    Ok(())
}

fn validate_identifier(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    validate_bounded_nonempty(field, value, max_bytes)?;
    if !value
        .bytes()
        .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"_.-".contains(&byte))
    {
        return Err(invalid(format!(
            "{field} 只能包含小写 ASCII、数字、下划线、点或连字符"
        )));
    }
    Ok(())
}

fn validate_stream_id(value: &str) -> Result<(), GenerationContractError> {
    validate_identifier("stream_id", value, MAX_STREAM_ID_BYTES)
}

fn validate_bounded_nonempty(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    if value.is_empty() || value.len() > max_bytes {
        return Err(invalid(format!(
            "{field} 必须为 1..={max_bytes} 个 UTF-8 字节"
        )));
    }
    Ok(())
}

fn validate_text(
    field: &str,
    value: &str,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    if value.len() > max_bytes {
        return Err(invalid(format!("{field} 超过 {max_bytes} 个 UTF-8 字节")));
    }
    Ok(())
}

fn validate_optional_text(
    field: &str,
    value: Option<&str>,
    max_bytes: usize,
) -> Result<(), GenerationContractError> {
    if let Some(value) = value {
        validate_text(field, value, max_bytes)?;
    }
    Ok(())
}

fn validate_sha256(field: &str, value: &str) -> Result<(), GenerationContractError> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    {
        return Err(invalid(format!("{field} 必须是 64 位小写 SHA-256 hex")));
    }
    Ok(())
}

fn validate_safe_u64(
    field: &str,
    value: u64,
    allow_zero: bool,
) -> Result<(), GenerationContractError> {
    if value > JS_SAFE_INTEGER_U64 || (!allow_zero && value == 0) {
        return Err(invalid(format!(
            "{field} 必须在 {}..={JS_SAFE_INTEGER_U64} 范围内",
            if allow_zero { 0 } else { 1 }
        )));
    }
    Ok(())
}

fn validate_safe_i64(field: &str, value: i64) -> Result<(), GenerationContractError> {
    if !(0..=JS_SAFE_INTEGER_I64).contains(&value) {
        return Err(invalid(format!(
            "{field} 必须在 0..={JS_SAFE_INTEGER_I64} 范围内"
        )));
    }
    Ok(())
}

fn validate_optional_safe_i64(
    field: &str,
    value: Option<i64>,
) -> Result<(), GenerationContractError> {
    if let Some(value) = value {
        validate_safe_i64(field, value)?;
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> GenerationContractError {
    GenerationContractError(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    const TASK_ID: &str = "550e8400-e29b-41d4-a716-446655440000";
    const SHA256_HELLO: &str = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";

    fn started(task_id: &str) -> StartedEvent {
        StartedEvent {
            task_kind: "llm.chat".to_string(),
            delivery_mode: GenerationDeliveryMode::Stream,
            project_id: Some(1),
            root_project_id: Some(1),
            workflow_run_id: None,
            generation_run_id: None,
            target: Some(GenerationTargetRef {
                kind: "chapter".to_string(),
                id: "42".to_string(),
                revision: Some("1".to_string()),
            }),
            batch_policy: None,
            root_task_id: task_id.to_string(),
            parent_task_id: None,
            item_key: None,
            retry_of_task_id: None,
        }
    }

    fn envelope(sequence: u64, event: GenerationEvent) -> GenerationTaskEvent {
        GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: TASK_ID.to_string(),
            attempt: 1,
            sequence,
            emitted_at_ms: 0,
            event,
        }
    }

    fn artifact(temporary: bool) -> GenerationArtifactRef {
        GenerationArtifactRef {
            artifact_id: "artifact-1".to_string(),
            artifact_kind: "image".to_string(),
            content_type: "image/png".to_string(),
            sha256: "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855".to_string(),
            byte_size: 0,
            revision: "1".to_string(),
            temporary,
            expires_at_ms: temporary.then_some(1_000),
        }
    }

    #[test]
    fn started_event_round_trips_and_validates() {
        let event = envelope(1, GenerationEvent::Started(started(TASK_ID)));
        let encoded = serde_json::to_vec(&event).expect("started event serializes");
        let decoded: GenerationTaskEvent =
            serde_json::from_slice(&encoded).expect("started event deserializes");
        assert_eq!(decoded, event);
        decoded.validate().expect("started event is valid");
    }

    #[test]
    fn event_validator_rejects_invalid_identity_and_sequence() {
        let mut invalid_uuid = envelope(1, GenerationEvent::Started(started(TASK_ID)));
        invalid_uuid.task_id = "00000000-0000-1000-8000-000000000000".to_string();
        assert!(invalid_uuid.validate().is_err());

        let invalid_sequence = envelope(0, GenerationEvent::Started(started(TASK_ID)));
        assert!(invalid_sequence.validate().is_err());
    }

    #[test]
    fn event_validator_rejects_usage_text_and_artifact_contract_errors() {
        let usage = GenerationUsage {
            prompt_tokens: 2,
            completion_tokens: 3,
            total_tokens: 4,
            prompt_cache_hit_tokens: None,
            prompt_cache_miss_tokens: None,
        };
        assert!(
            envelope(
                2,
                GenerationEvent::Usage(UsageEvent {
                    usage,
                    cumulative: true,
                }),
            )
            .validate()
            .is_err()
        );

        let text = envelope(
            2,
            GenerationEvent::TextDelta(TextDeltaEvent {
                stream_id: "raw_json".to_string(),
                offset: JS_SAFE_INTEGER_U64 + 1,
                delta: "x".to_string(),
                chunk_index: 0,
            }),
        );
        assert!(text.validate().is_err());

        let invalid_preview = envelope(
            2,
            GenerationEvent::Artifact(ArtifactEvent {
                artifact: artifact(false),
                operation: ArtifactOperation::PreviewAvailable,
                replaces_artifact_id: None,
                item_key: None,
                item_index: None,
                item_total: None,
            }),
        );
        assert!(invalid_preview.validate().is_err());
    }

    #[test]
    fn snapshot_rejects_text_hash_offset_and_terminal_status_mismatch() {
        let stream = TextStreamSnapshot {
            stream_id: "raw_json".to_string(),
            text: "hello".to_string(),
            next_offset: 4,
            sha256: SHA256_HELLO.to_string(),
        };
        let failed = GenerationTaskEvent {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: TASK_ID.to_string(),
            attempt: 1,
            sequence: 2,
            emitted_at_ms: 0,
            event: GenerationEvent::Failed(FailedEvent {
                error: GenerationError {
                    code: GenerationErrorCode::registered("generation_event_invalid").unwrap(),
                    message: "invalid".to_string(),
                    retryable: false,
                    stage_code: "validate".to_string(),
                    trace_id: None,
                    source_error: None,
                },
                draft_disposition: DraftDisposition::Discarded,
                batch_summary: None,
            }),
        };
        let snapshot = GenerationTaskSnapshot {
            schema_version: GENERATION_EVENT_SCHEMA_VERSION,
            task_id: TASK_ID.to_string(),
            attempt: 1,
            covers_through_sequence: 2,
            status: GenerationTaskStatus::Completed,
            started: started(TASK_ID),
            cancel_reason: None,
            progress: None,
            streams: vec![stream],
            artifacts: vec![artifact(false)],
            usage: None,
            latest_checkpoint: None,
            terminal_event: Some(failed),
            updated_at_ms: 0,
        };
        assert!(snapshot.validate().is_err());
    }

    #[test]
    fn unknown_error_codes_are_rejected_at_deserialization() {
        let error = serde_json::from_value::<GenerationErrorCode>(json!("temporary_unknown"))
            .expect_err("unregistered codes must not enter the wire contract");
        assert!(error.to_string().contains("未注册的生成错误码"));
    }

    #[test]
    fn rust_accepts_checked_generation_fixtures_and_utf8_offsets() {
        for fixture in [
            include_str!("../../fixtures/generation-events-v1/started.json"),
            include_str!("../../fixtures/generation-events-v1/text-delta-mixed-unicode.json"),
            include_str!("../../fixtures/generation-events-v1/text-delta-second-chunk.json"),
        ] {
            let event: GenerationTaskEvent =
                serde_json::from_str(fixture).expect("fixture parses as GenerationTaskEvent");
            event
                .validate()
                .expect("fixture satisfies generation contract");
        }

        #[derive(Deserialize)]
        struct Utf8Vector {
            text: String,
            utf8_bytes: usize,
        }

        let vectors: Vec<Utf8Vector> = serde_json::from_str(include_str!(
            "../../fixtures/generation-events-v1/utf8-vectors.json"
        ))
        .expect("UTF-8 vectors parse");
        assert!(
            vectors
                .iter()
                .all(|vector| vector.text.len() == vector.utf8_bytes)
        );
    }

    #[test]
    fn generation_schema_is_strict_and_marks_safe_integer_boundaries() {
        let schema = serde_json::to_value(schemars::schema_for!(GenerationTaskEvent))
            .expect("schema serializes");
        assert_eq!(
            schema.get("$schema").and_then(Value::as_str),
            Some("https://json-schema.org/draft/2020-12/schema")
        );
        assert_eq!(
            schema.get("additionalProperties").and_then(Value::as_bool),
            Some(false)
        );
        let json_text = schema.to_string();
        assert!(json_text.contains("9007199254740991"));
    }
}
