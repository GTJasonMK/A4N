use a4n_db_models::models::{
    generation::{
        generation_run, generation_start_request, generation_task_checkpoint, workflow_run,
    },
    task::task_queue,
};
use a4n_domain::generation::{
    GenerationTaskCheckpointRecord, GenerationTaskCheckpointSummaryRecord, GenerationTaskListCursor,
};
use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority};
use a4n_service_protocol::{
    BatchCommitPolicy, DEFAULT_GENERATION_ACTIVE_SESSION_MAX, GENERATION_EVENT_SCHEMA_VERSION,
    GenerationDeliveryMode, GenerationTargetRef, GenerationTaskSnapshot, GenerationTaskStatus,
    JS_SAFE_INTEGER_I64, StartedEvent,
};
use chrono::{DateTime, NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    DbBackend, DbErr, EntityTrait, FromQueryResult, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, Statement, TransactionTrait,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use uuid::{Uuid, Version};

use super::{
    canonical_json_sha256, canonical_json_text,
    lifecycle_gate::{require_generation_owner_gates_active, resolve_generation_root_project_id},
    verify_canonical_json_sha256,
};
use crate::workflow::WORKFLOW_STATUS_COMPLETED;

mod checkpoint;
mod history;
mod reservation;

pub use checkpoint::{
    bind_generation_task_to_workflow, extend_generation_sequence_lease, load_generation_task,
    load_generation_task_checkpoint, persist_generation_checkpoint,
};
#[cfg(test)]
use history::generation_public_snapshot_for_summary;
pub use history::{
    generation_public_snapshot_for_summary_record, list_active_generation_task_checkpoints,
    list_generation_task_history_checkpoints,
};
pub use reservation::reserve_generation_task;

pub const GENERATION_SEQUENCE_LEASE_SIZE: i64 = 256;
const INITIAL_RESERVED_THROUGH_SEQUENCE: i64 = 257;
const MAX_GENERATION_START_REQUEST_MAPPINGS: u64 = 16;
const MAX_ACTIVE_GENERATION_TASKS: u32 = DEFAULT_GENERATION_ACTIVE_SESSION_MAX as u32;
const MAX_GENERATION_HISTORY_LIMIT: u32 = 100;

const START_REQUEST_LOCK_NAMESPACE: i32 = 1_093_948_242;
const BUSINESS_ATTEMPT_LOCK_NAMESPACE: i32 = 1_093_943_636;
const ACTIVE_GENERATION_STATUSES: [&str; 4] = ["reserved", "running", "finalizing", "cancelling"];
const HISTORY_GENERATION_STATUSES: [&str; 4] = ["completed", "failed", "cancelled", "interrupted"];

const GENERATION_UPDATED_AT_MS_SQL: &str =
    "FLOOR(EXTRACT(EPOCH FROM (checkpoint.updated_at AT TIME ZONE 'UTC')) * 1000)::BIGINT";

const EXTEND_SEQUENCE_LEASE_SQL: &str = r#"
UPDATE generation_task_checkpoint
SET reserved_through_sequence = reserved_through_sequence + $4,
    checkpoint_revision = checkpoint_revision + 1,
    updated_at = NOW()
WHERE task_id = $1
  AND checkpoint_revision = $2
  AND reserved_through_sequence = $3
  AND status IN ('reserved', 'running', 'finalizing', 'cancelling')
  AND checkpoint_revision < $5
  AND reserved_through_sequence <= $5 - $4
RETURNING checkpoint_revision, reserved_through_sequence
"#;

const PERSIST_CHECKPOINT_SQL: &str = r#"
UPDATE generation_task_checkpoint
SET public_snapshot_json = $5,
    public_snapshot_sha256 = $6,
    covers_through_sequence = $7,
    step_key = COALESCE($8, step_key),
    call_key = COALESCE($9, call_key),
    provider_kind = COALESCE($10, provider_kind),
    provider_model = COALESCE($11, provider_model),
    provider_job_id = COALESCE($12, provider_job_id),
    provider_idempotency_key_sha256 = COALESCE($13, provider_idempotency_key_sha256),
    resume_metadata_version = COALESCE($14, resume_metadata_version),
    execution_checkpoint_json = COALESCE($15, execution_checkpoint_json),
    execution_checkpoint_sha256 = COALESCE($16, execution_checkpoint_sha256),
    checkpoint_revision = checkpoint_revision + 1,
    updated_at = NOW()
WHERE task_id = $1
  AND attempt = $2
  AND checkpoint_revision = $3
  AND status = $4
  AND status IN ('reserved', 'running', 'cancelling')
  AND covers_through_sequence <= $7
  AND reserved_through_sequence >= $7
  AND checkpoint_revision < $17
RETURNING checkpoint_revision, covers_through_sequence, reserved_through_sequence, updated_at
"#;

#[derive(Debug, Clone)]
pub struct ReserveGenerationTaskInput {
    pub requested_by_user_id: i64,
    pub start_request_id: String,
    pub business_key_json: Value,
    pub input_fingerprint_json: Value,
    pub task_kind: TaskKind,
    pub priority: TaskPriority,
    pub delivery_mode: GenerationDeliveryMode,
    pub batch_policy: Option<BatchCommitPolicy>,
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
    pub workflow_run_id: Option<i64>,
    pub generation_run_id: Option<i64>,
    pub target: Option<GenerationTargetRef>,
    pub root_task_id: Option<String>,
    pub parent_task_id: Option<String>,
    pub item_key: Option<String>,
    pub retry_of_task_id: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PersistedGenerationTask {
    pub checkpoint: GenerationTaskCheckpointRecord,
    pub public_snapshot: Option<GenerationTaskSnapshot>,
}

#[derive(Debug, Clone)]
pub enum GenerationPublicSnapshot {
    Available(GenerationTaskSnapshot),
    Expired,
    Corrupt,
}

#[derive(Debug, Clone)]
pub struct GenerationTaskCheckpointPage {
    pub items: Vec<GenerationTaskCheckpointSummaryRecord>,
    pub has_more: bool,
}

#[derive(Debug, FromQueryResult)]
struct GenerationTaskSummaryRow {
    task_id: String,
    attempt: i32,
    requested_by_user_id: i64,
    task_kind: String,
    delivery_mode: String,
    batch_policy: Option<String>,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    generation_run_id: Option<i64>,
    target_kind: Option<String>,
    target_id: Option<String>,
    target_revision: Option<String>,
    root_task_id: String,
    parent_task_id: Option<String>,
    item_key: Option<String>,
    retry_of_task_id: Option<String>,
    status: String,
    covers_through_sequence: i64,
    public_snapshot_json: Option<Value>,
    public_snapshot_sha256: Option<String>,
    result_kind: Option<String>,
    result_id: Option<String>,
    result_revision: Option<String>,
    expires_at: Option<NaiveDateTime>,
    updated_at: NaiveDateTime,
}

#[derive(Debug, FromQueryResult)]
struct GenerationTaskLineageRow {
    task_id: String,
    business_key: String,
    attempt: i32,
    requested_by_user_id: i64,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    root_task_id: String,
    parent_task_id: Option<String>,
    item_key: Option<String>,
}

impl GenerationTaskSummaryRow {
    fn into_checkpoint(self) -> GenerationTaskCheckpointSummaryRecord {
        GenerationTaskCheckpointSummaryRecord {
            task_id: self.task_id,
            attempt: self.attempt,
            requested_by_user_id: self.requested_by_user_id,
            task_kind: self.task_kind,
            delivery_mode: self.delivery_mode,
            batch_policy: self.batch_policy,
            project_id: self.project_id,
            root_project_id: self.root_project_id,
            workflow_run_id: self.workflow_run_id,
            generation_run_id: self.generation_run_id,
            target_kind: self.target_kind,
            target_id: self.target_id,
            target_revision: self.target_revision,
            root_task_id: self.root_task_id,
            parent_task_id: self.parent_task_id,
            item_key: self.item_key,
            retry_of_task_id: self.retry_of_task_id,
            status: self.status,
            covers_through_sequence: self.covers_through_sequence,
            public_snapshot_json: self.public_snapshot_json,
            public_snapshot_sha256: self.public_snapshot_sha256,
            result_kind: self.result_kind,
            result_id: self.result_id,
            result_revision: self.result_revision,
            expires_at: self.expires_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Clone)]
pub struct GenerationTaskReservation {
    pub created: bool,
    pub task: PersistedGenerationTask,
    pub queue_task: TaskInfo,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenerationSequenceLease {
    pub checkpoint_revision: i64,
    pub reserved_through_sequence: i64,
}

#[derive(Debug, Clone)]
pub struct PersistGenerationCheckpointInput {
    pub task_id: String,
    pub attempt: u32,
    pub expected_checkpoint_revision: i64,
    pub public_snapshot: GenerationTaskSnapshot,
    pub step_key: Option<String>,
    pub call_key: Option<String>,
    pub provider_kind: Option<String>,
    pub provider_model: Option<String>,
    pub provider_job_id: Option<String>,
    pub provider_idempotency_key_sha256: Option<String>,
    pub resume_metadata_version: Option<i32>,
    pub execution_checkpoint_json: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedGenerationCheckpoint {
    pub checkpoint_revision: i64,
    pub covers_through_sequence: i64,
    pub reserved_through_sequence: i64,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationTaskPersistenceError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{message}")]
    Rejected { code: &'static str, message: String },
    #[error("{0}")]
    NotFound(String),
    #[error("{0}")]
    Corrupt(String),
    #[error("sequence lease CAS 失败")]
    LeaseConflict,
    #[error("checkpoint CAS 失败")]
    CheckpointConflict,
    #[error(transparent)]
    Json(#[from] serde_json::Error),
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl GenerationTaskPersistenceError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidArgument(_) | Self::Json(_) => "generation_request_invalid",
            Self::Rejected { code, .. } => code,
            Self::NotFound(_) => "generation_task_not_found",
            Self::Corrupt(_) => "generation_checkpoint_corrupt",
            Self::LeaseConflict | Self::CheckpointConflict | Self::Database(_) => {
                "generation_persist_failed"
            }
        }
    }
}

fn verified_generation_public_snapshot(
    checkpoint: &generation_task_checkpoint::Model,
) -> Result<Option<GenerationTaskSnapshot>, GenerationTaskPersistenceError> {
    if !ACTIVE_GENERATION_STATUSES.contains(&checkpoint.status.as_str())
        && checkpoint
            .expires_at
            .is_some_and(|expires_at| expires_at <= Utc::now().naive_utc())
    {
        // TTL 到期后正文即使尚未被 cleanup worker 清空，也不能继续作为可用 snapshot 返回。
        return Ok(None);
    }
    let (snapshot_json, expected_sha256) = match (
        checkpoint.public_snapshot_json.as_ref(),
        checkpoint.public_snapshot_sha256.as_deref(),
    ) {
        (Some(snapshot_json), Some(expected_sha256)) => (snapshot_json, expected_sha256),
        (None, Some(_))
            if !ACTIVE_GENERATION_STATUSES.contains(&checkpoint.status.as_str())
                && checkpoint
                    .expires_at
                    .is_some_and(|expires_at| expires_at <= Utc::now().naive_utc()) =>
        {
            // Terminal snapshot 正文到期后只保留 hash/result 规范列，summary 查询仍可用。
            return Ok(None);
        }
        (None, None) if !ACTIVE_GENERATION_STATUSES.contains(&checkpoint.status.as_str()) => {
            return Err(corrupt(
                checkpoint,
                "terminal task 缺少 public snapshot hash",
            ));
        }
        _ => return Err(corrupt(checkpoint, "public snapshot JSON/hash 不成对")),
    };

    if !verify_canonical_json_sha256(snapshot_json, expected_sha256)? {
        return Err(corrupt(checkpoint, "public snapshot SHA-256 不匹配"));
    }
    let snapshot: GenerationTaskSnapshot = serde_json::from_value(snapshot_json.clone())
        .map_err(|error| corrupt(checkpoint, format!("public snapshot 解析失败: {error}")))?;
    snapshot
        .validate()
        .map_err(|error| corrupt(checkpoint, error.to_string()))?;
    validate_snapshot_matches_checkpoint(checkpoint, &snapshot)?;
    Ok(Some(snapshot))
}

fn verified_generation_execution_checkpoint(
    checkpoint: &generation_task_checkpoint::Model,
) -> Result<Option<Value>, GenerationTaskPersistenceError> {
    let (Some(checkpoint_json), Some(expected_sha256), Some(version)) = (
        checkpoint.execution_checkpoint_json.as_ref(),
        checkpoint.execution_checkpoint_sha256.as_deref(),
        checkpoint.resume_metadata_version,
    ) else {
        if checkpoint.execution_checkpoint_json.is_some()
            || checkpoint.execution_checkpoint_sha256.is_some()
            || checkpoint.resume_metadata_version.is_some()
        {
            return Err(corrupt(
                checkpoint,
                "execution checkpoint JSON/hash/version 不成组",
            ));
        }
        return Ok(None);
    };
    if version <= 0 {
        return Err(corrupt(
            checkpoint,
            "execution checkpoint version 必须大于 0",
        ));
    }
    if !verify_canonical_json_sha256(checkpoint_json, expected_sha256)? {
        return Err(corrupt(checkpoint, "execution checkpoint SHA-256 不匹配"));
    }
    Ok(Some(checkpoint_json.clone()))
}

pub(crate) fn persisted_generation_task_from_checkpoint(
    checkpoint: generation_task_checkpoint::Model,
) -> Result<PersistedGenerationTask, GenerationTaskPersistenceError> {
    let public_snapshot = verified_generation_public_snapshot(&checkpoint)?;
    verified_generation_execution_checkpoint(&checkpoint)?;
    Ok(PersistedGenerationTask {
        checkpoint: generation_task_checkpoint_record(checkpoint),
        public_snapshot,
    })
}

pub fn persisted_generation_task_from_record(
    checkpoint: GenerationTaskCheckpointRecord,
) -> Result<PersistedGenerationTask, GenerationTaskPersistenceError> {
    let model = generation_task_checkpoint_model(&checkpoint);
    let public_snapshot = verified_generation_public_snapshot(&model)?;
    verified_generation_execution_checkpoint(&model)?;
    Ok(PersistedGenerationTask {
        checkpoint,
        public_snapshot,
    })
}

pub fn started_event_from_record(
    checkpoint: &GenerationTaskCheckpointRecord,
) -> Result<StartedEvent, GenerationTaskPersistenceError> {
    started_event_from_checkpoint(&generation_task_checkpoint_model(checkpoint))
}

pub fn started_event_from_summary_record(
    checkpoint: &GenerationTaskCheckpointSummaryRecord,
) -> Result<StartedEvent, GenerationTaskPersistenceError> {
    started_event_from_checkpoint(&generation_task_summary_model(checkpoint))
}

pub(super) fn generation_task_checkpoint_record(
    model: generation_task_checkpoint::Model,
) -> GenerationTaskCheckpointRecord {
    GenerationTaskCheckpointRecord {
        task_id: model.task_id,
        business_key: model.business_key,
        attempt: model.attempt,
        requested_by_user_id: model.requested_by_user_id,
        task_kind: model.task_kind,
        delivery_mode: model.delivery_mode,
        batch_policy: model.batch_policy,
        project_id: model.project_id,
        root_project_id: model.root_project_id,
        workflow_run_id: model.workflow_run_id,
        generation_run_id: model.generation_run_id,
        target_kind: model.target_kind,
        target_id: model.target_id,
        target_revision: model.target_revision,
        root_task_id: model.root_task_id,
        parent_task_id: model.parent_task_id,
        item_key: model.item_key,
        retry_of_task_id: model.retry_of_task_id,
        status: model.status,
        checkpoint_revision: model.checkpoint_revision,
        covers_through_sequence: model.covers_through_sequence,
        reserved_through_sequence: model.reserved_through_sequence,
        public_snapshot_json: model.public_snapshot_json,
        public_snapshot_sha256: model.public_snapshot_sha256,
        step_key: model.step_key,
        call_key: model.call_key,
        provider_kind: model.provider_kind,
        provider_model: model.provider_model,
        provider_job_id: model.provider_job_id,
        provider_idempotency_key_sha256: model.provider_idempotency_key_sha256,
        resume_metadata_version: model.resume_metadata_version,
        execution_checkpoint_json: model.execution_checkpoint_json,
        execution_checkpoint_sha256: model.execution_checkpoint_sha256,
        input_fingerprint_sha256: model.input_fingerprint_sha256,
        result_kind: model.result_kind,
        result_id: model.result_id,
        result_revision: model.result_revision,
        expires_at: model.expires_at,
        created_at: model.created_at,
        updated_at: model.updated_at,
        completed_at: model.completed_at,
    }
}

fn generation_task_checkpoint_model(
    record: &GenerationTaskCheckpointRecord,
) -> generation_task_checkpoint::Model {
    generation_task_checkpoint::Model {
        task_id: record.task_id.clone(),
        business_key: record.business_key.clone(),
        attempt: record.attempt,
        requested_by_user_id: record.requested_by_user_id,
        task_kind: record.task_kind.clone(),
        delivery_mode: record.delivery_mode.clone(),
        batch_policy: record.batch_policy.clone(),
        project_id: record.project_id,
        root_project_id: record.root_project_id,
        workflow_run_id: record.workflow_run_id,
        generation_run_id: record.generation_run_id,
        target_kind: record.target_kind.clone(),
        target_id: record.target_id.clone(),
        target_revision: record.target_revision.clone(),
        root_task_id: record.root_task_id.clone(),
        parent_task_id: record.parent_task_id.clone(),
        item_key: record.item_key.clone(),
        retry_of_task_id: record.retry_of_task_id.clone(),
        status: record.status.clone(),
        checkpoint_revision: record.checkpoint_revision,
        covers_through_sequence: record.covers_through_sequence,
        reserved_through_sequence: record.reserved_through_sequence,
        public_snapshot_json: record.public_snapshot_json.clone(),
        public_snapshot_sha256: record.public_snapshot_sha256.clone(),
        step_key: record.step_key.clone(),
        call_key: record.call_key.clone(),
        provider_kind: record.provider_kind.clone(),
        provider_model: record.provider_model.clone(),
        provider_job_id: record.provider_job_id.clone(),
        provider_idempotency_key_sha256: record.provider_idempotency_key_sha256.clone(),
        resume_metadata_version: record.resume_metadata_version,
        execution_checkpoint_json: record.execution_checkpoint_json.clone(),
        execution_checkpoint_sha256: record.execution_checkpoint_sha256.clone(),
        input_fingerprint_sha256: record.input_fingerprint_sha256.clone(),
        result_kind: record.result_kind.clone(),
        result_id: record.result_id.clone(),
        result_revision: record.result_revision.clone(),
        expires_at: record.expires_at,
        created_at: record.created_at,
        updated_at: record.updated_at,
        completed_at: record.completed_at,
    }
}

fn generation_task_summary_model(
    record: &GenerationTaskCheckpointSummaryRecord,
) -> generation_task_checkpoint::Model {
    generation_task_checkpoint::Model {
        task_id: record.task_id.clone(),
        business_key: String::new(),
        attempt: record.attempt,
        requested_by_user_id: record.requested_by_user_id,
        task_kind: record.task_kind.clone(),
        delivery_mode: record.delivery_mode.clone(),
        batch_policy: record.batch_policy.clone(),
        project_id: record.project_id,
        root_project_id: record.root_project_id,
        workflow_run_id: record.workflow_run_id,
        generation_run_id: record.generation_run_id,
        target_kind: record.target_kind.clone(),
        target_id: record.target_id.clone(),
        target_revision: record.target_revision.clone(),
        root_task_id: record.root_task_id.clone(),
        parent_task_id: record.parent_task_id.clone(),
        item_key: record.item_key.clone(),
        retry_of_task_id: record.retry_of_task_id.clone(),
        status: record.status.clone(),
        checkpoint_revision: 0,
        covers_through_sequence: record.covers_through_sequence,
        reserved_through_sequence: 0,
        public_snapshot_json: record.public_snapshot_json.clone(),
        public_snapshot_sha256: record.public_snapshot_sha256.clone(),
        step_key: None,
        call_key: None,
        provider_kind: None,
        provider_model: None,
        provider_job_id: None,
        provider_idempotency_key_sha256: None,
        resume_metadata_version: None,
        execution_checkpoint_json: None,
        execution_checkpoint_sha256: None,
        input_fingerprint_sha256: String::new(),
        result_kind: record.result_kind.clone(),
        result_id: record.result_id.clone(),
        result_revision: record.result_revision.clone(),
        expires_at: record.expires_at,
        created_at: record.updated_at,
        updated_at: record.updated_at,
        completed_at: None,
    }
}

fn validate_snapshot_matches_checkpoint(
    checkpoint: &generation_task_checkpoint::Model,
    snapshot: &GenerationTaskSnapshot,
) -> Result<(), GenerationTaskPersistenceError> {
    let expected_attempt = u32::try_from(checkpoint.attempt)
        .map_err(|_| corrupt(checkpoint, "attempt 超出 u32 范围"))?;
    let expected_covers = u64::try_from(checkpoint.covers_through_sequence)
        .map_err(|_| corrupt(checkpoint, "covers_through_sequence 为负数"))?;
    let expected_status: GenerationTaskStatus = enum_from_storage(&checkpoint.status, "status")
        .map_err(|error| corrupt(checkpoint, error.to_string()))?;
    let expected_started = started_event_from_checkpoint(checkpoint)?;
    if snapshot.task_id != checkpoint.task_id
        || snapshot.attempt != expected_attempt
        || snapshot.covers_through_sequence != expected_covers
        || snapshot.status != expected_status
        || snapshot.started != expected_started
    {
        return Err(corrupt(
            checkpoint,
            "public snapshot 与 checkpoint 规范列不一致",
        ));
    }
    Ok(())
}

pub(crate) fn started_event_from_checkpoint(
    checkpoint: &generation_task_checkpoint::Model,
) -> Result<StartedEvent, GenerationTaskPersistenceError> {
    let target = match (
        checkpoint.target_kind.as_ref(),
        checkpoint.target_id.as_ref(),
    ) {
        (None, None) if checkpoint.target_revision.is_none() => None,
        (Some(kind), Some(id)) => Some(GenerationTargetRef {
            kind: kind.clone(),
            id: id.clone(),
            revision: checkpoint.target_revision.clone(),
        }),
        _ => return Err(corrupt(checkpoint, "target 规范列不成组")),
    };
    Ok(StartedEvent {
        task_kind: checkpoint.task_kind.clone(),
        delivery_mode: enum_from_storage(&checkpoint.delivery_mode, "delivery_mode")
            .map_err(|error| corrupt(checkpoint, error.to_string()))?,
        project_id: checkpoint.project_id,
        root_project_id: checkpoint.root_project_id,
        workflow_run_id: checkpoint.workflow_run_id,
        generation_run_id: checkpoint.generation_run_id,
        target,
        batch_policy: checkpoint
            .batch_policy
            .as_deref()
            .map(|value| enum_from_storage(value, "batch_policy"))
            .transpose()
            .map_err(|error| corrupt(checkpoint, error.to_string()))?,
        root_task_id: checkpoint.root_task_id.clone(),
        parent_task_id: checkpoint.parent_task_id.clone(),
        item_key: checkpoint.item_key.clone(),
        retry_of_task_id: checkpoint.retry_of_task_id.clone(),
    })
}

pub(super) fn enum_storage<T>(
    value: T,
    label: &str,
) -> Result<String, GenerationTaskPersistenceError>
where
    T: Serialize,
{
    let value = serde_json::to_value(value)?;
    value.as_str().map(str::to_string).ok_or_else(|| {
        GenerationTaskPersistenceError::InvalidArgument(format!(
            "{label} 不能序列化为 storage wire"
        ))
    })
}

pub(super) fn enum_from_storage<T>(
    value: &str,
    label: &str,
) -> Result<T, GenerationTaskPersistenceError>
where
    T: DeserializeOwned,
{
    serde_json::from_value(Value::String(value.to_string())).map_err(|error| {
        GenerationTaskPersistenceError::Corrupt(format!("{label} storage wire 无效: {error}"))
    })
}

fn parse_uuid_v4(value: &str, label: &str) -> Result<Uuid, GenerationTaskPersistenceError> {
    let uuid = Uuid::parse_str(value).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument(format!("{label} 必须是 UUID"))
    })?;
    if uuid.get_version() != Some(Version::Random) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "{label} 必须是 UUID v4"
        )));
    }
    Ok(uuid)
}

pub(super) fn canonical_uuid_v4(
    value: &str,
    label: &str,
) -> Result<String, GenerationTaskPersistenceError> {
    Ok(parse_uuid_v4(value, label)?.to_string())
}

fn normalize_optional_uuid(
    value: Option<String>,
    label: &str,
) -> Result<Option<String>, GenerationTaskPersistenceError> {
    value
        .map(|value| canonical_uuid_v4(value.trim(), label))
        .transpose()
}

fn normalize_optional_text(
    value: Option<String>,
    label: &str,
    max_bytes: usize,
) -> Result<Option<String>, GenerationTaskPersistenceError> {
    value
        .map(|value| required_text(value, label, max_bytes))
        .transpose()
}

fn validate_optional_sha256(
    value: Option<String>,
    label: &str,
) -> Result<Option<String>, GenerationTaskPersistenceError> {
    value
        .map(|value| {
            let value = value.trim().to_string();
            if value.len() == 64
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
            {
                Ok(value)
            } else {
                Err(GenerationTaskPersistenceError::InvalidArgument(format!(
                    "{label} 必须是 64 位小写 SHA-256 hex"
                )))
            }
        })
        .transpose()
}

fn required_text(
    value: String,
    label: &str,
    max_bytes: usize,
) -> Result<String, GenerationTaskPersistenceError> {
    let value = value.trim().to_string();
    if value.is_empty() || value.len() > max_bytes {
        Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "{label} 必须在 1..={max_bytes} 字节内"
        )))
    } else {
        Ok(value)
    }
}

pub(super) fn validate_safe_id(
    value: i64,
    label: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    if (1..=JS_SAFE_INTEGER_I64).contains(&value) {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "{label} 必须在 JavaScript safe positive integer 范围内"
        )))
    }
}

fn validate_optional_safe_id(
    value: Option<i64>,
    label: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    value
        .map(|value| validate_safe_id(value, label))
        .unwrap_or(Ok(()))
}

pub(super) fn validate_safe_sequence(
    value: i64,
    label: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    validate_safe_id(value, label)
}

fn corrupt(
    checkpoint: &generation_task_checkpoint::Model,
    message: impl Into<String>,
) -> GenerationTaskPersistenceError {
    GenerationTaskPersistenceError::Corrupt(format!(
        "generation task {} checkpoint 损坏: {}",
        checkpoint.task_id,
        message.into()
    ))
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;

    use super::*;

    fn reservation_input() -> ReserveGenerationTaskInput {
        ReserveGenerationTaskInput {
            requested_by_user_id: 7,
            start_request_id: "f1c30de8-8a81-4bf2-bb6b-11957c30fd35".to_string(),
            business_key_json: json!({
                "version": 1,
                "task_kind": "text_generation",
                "project_id": 11,
                "target": {"kind": "chapter", "id": "9"},
            }),
            input_fingerprint_json: json!({
                "chapter_id": 9,
                "prompt": "稳定输入",
            }),
            task_kind: TaskKind::TextGeneration,
            priority: TaskPriority::High,
            delivery_mode: GenerationDeliveryMode::Stream,
            batch_policy: None,
            project_id: Some(11),
            root_project_id: Some(11),
            source_snapshot_id: None,
            task_scope: Some("text_generation".to_string()),
            workflow_run_id: None,
            generation_run_id: None,
            target: Some(GenerationTargetRef {
                kind: "chapter".to_string(),
                id: "9".to_string(),
                revision: None,
            }),
            root_task_id: None,
            parent_task_id: None,
            item_key: None,
            retry_of_task_id: None,
        }
    }

    #[test]
    fn reservation_builds_started_snapshot_and_first_sequence_lease() {
        let prepared =
            reservation::prepare_reservation(reservation_input()).expect("prepared reservation");
        let now = Utc.timestamp_millis_opt(1_750_000_000_000).unwrap();
        let (checkpoint, snapshot) = reservation::build_reserved_checkpoint(
            &prepared,
            "67263d6c-7557-4d2f-a75f-4cd5b1c1836a".to_string(),
            1,
            now,
        )
        .expect("reserved checkpoint");

        assert_eq!(checkpoint.root_task_id, checkpoint.task_id);
        assert_eq!(checkpoint.covers_through_sequence, 1);
        assert_eq!(
            checkpoint.reserved_through_sequence,
            INITIAL_RESERVED_THROUGH_SEQUENCE
        );
        assert_eq!(checkpoint.checkpoint_revision, 1);
        assert_eq!(snapshot.status, GenerationTaskStatus::Reserved);
        assert_eq!(snapshot.started.root_project_id, Some(11));
        assert_eq!(
            verified_generation_public_snapshot(&checkpoint).unwrap(),
            Some(snapshot)
        );

        let mut corrupt_checkpoint = checkpoint;
        corrupt_checkpoint.public_snapshot_sha256 = Some("0".repeat(64));
        assert!(matches!(
            verified_generation_public_snapshot(&corrupt_checkpoint),
            Err(GenerationTaskPersistenceError::Corrupt(_))
        ));
        assert!(matches!(
            generation_public_snapshot_for_summary(&corrupt_checkpoint),
            GenerationPublicSnapshot::Corrupt
        ));

        let mut expired_checkpoint = corrupt_checkpoint;
        expired_checkpoint.status = "completed".to_string();
        expired_checkpoint.expires_at = Some(now.naive_utc() - chrono::Duration::seconds(1));
        assert!(matches!(
            generation_public_snapshot_for_summary(&expired_checkpoint),
            GenerationPublicSnapshot::Expired
        ));
    }

    #[test]
    fn advisory_lock_hashes_follow_fixed_big_endian_sha256_prefixes() {
        let request_id = Uuid::parse_str("f1c30de8-8a81-4bf2-bb6b-11957c30fd35").expect("uuid");
        assert_eq!(
            reservation::start_request_lock_hash(7, request_id),
            1_496_059_712
        );
        assert_eq!(
            reservation::business_attempt_lock_hash("{\"project_id\":11,\"version\":1}"),
            547_212_904
        );
    }

    #[test]
    fn sequence_lease_sql_is_single_active_high_water_cas() {
        for required in [
            "checkpoint_revision = $2",
            "reserved_through_sequence = $3",
            "reserved_through_sequence + $4",
            "checkpoint_revision + 1",
            "status IN ('reserved', 'running', 'finalizing', 'cancelling')",
            "RETURNING checkpoint_revision, reserved_through_sequence",
        ] {
            assert!(
                EXTEND_SEQUENCE_LEASE_SQL.contains(required),
                "lease SQL missing {required}"
            );
        }
    }

    #[test]
    fn checkpoint_sql_is_single_snapshot_cas_and_preserves_omitted_metadata() {
        for required in [
            "checkpoint_revision = $3",
            "status = $4",
            "covers_through_sequence <= $7",
            "reserved_through_sequence >= $7",
            "checkpoint_revision = checkpoint_revision + 1",
            "step_key = COALESCE($8, step_key)",
            "execution_checkpoint_json = COALESCE($15, execution_checkpoint_json)",
            "RETURNING checkpoint_revision, covers_through_sequence, reserved_through_sequence, updated_at",
        ] {
            assert!(
                PERSIST_CHECKPOINT_SQL.contains(required),
                "checkpoint SQL missing {required}"
            );
        }
        assert!(validate_optional_sha256(Some("a".repeat(64)), "hash").is_ok());
        assert!(validate_optional_sha256(Some("A".repeat(64)), "hash").is_err());
    }
}
