use a4n_core::models::{
    generation::{
        generation_run, generation_start_request, generation_task_checkpoint, workflow_run,
    },
    task::task_queue,
};
use a4n_queue::priority::{TaskInfo, TaskKind, TaskPriority};
use a4n_service_protocol::{
    BatchCommitPolicy, DEFAULT_GENERATION_ACTIVE_SESSION_MAX, GENERATION_EVENT_SCHEMA_VERSION,
    GenerationDeliveryMode, GenerationTargetRef, GenerationTaskSnapshot, GenerationTaskStatus,
    JS_SAFE_INTEGER_I64, StartedEvent,
};
use chrono::{DateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    DbBackend, DbErr, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, QuerySelect, Set,
    Statement, TransactionTrait,
};
use serde::{Serialize, de::DeserializeOwned};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use uuid::{Uuid, Version};

use super::{
    canonical_json_sha256, canonical_json_text,
    lifecycle_gate::{require_generation_owner_gates_active, resolve_generation_root_project_id},
    verify_canonical_json_sha256,
};

pub const GENERATION_SEQUENCE_LEASE_SIZE: i64 = 256;
pub const INITIAL_RESERVED_THROUGH_SEQUENCE: i64 = 257;
pub const MAX_GENERATION_START_REQUEST_MAPPINGS: u64 = 16;
pub const MAX_ACTIVE_GENERATION_TASKS: u32 = DEFAULT_GENERATION_ACTIVE_SESSION_MAX as u32;
pub const MAX_GENERATION_HISTORY_LIMIT: u32 = 100;

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
    pub checkpoint: generation_task_checkpoint::Model,
    pub public_snapshot: Option<GenerationTaskSnapshot>,
}

#[derive(Debug, Clone)]
pub enum GenerationPublicSnapshot {
    Available(GenerationTaskSnapshot),
    Expired,
    Corrupt,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationTaskListCursor {
    pub updated_at_ms: i64,
    pub task_id: String,
}

#[derive(Debug, Clone)]
pub struct GenerationTaskCheckpointPage {
    pub items: Vec<generation_task_checkpoint::Model>,
    pub has_more: bool,
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

struct PreparedReservation {
    input: ReserveGenerationTaskInput,
    start_request_id: Uuid,
    business_key: String,
    input_fingerprint_sha256: String,
}

pub async fn reserve_generation_task(
    db: &DatabaseConnection,
    input: ReserveGenerationTaskInput,
) -> Result<GenerationTaskReservation, GenerationTaskPersistenceError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "生成任务 reservation 仅支持 PostgreSQL".to_string(),
        ));
    }
    let mut prepared = prepare_reservation(input)?;
    if let Some(project_id) = prepared.input.project_id {
        let canonical_root_project_id = resolve_generation_root_project_id(db, project_id).await?;
        if let Some(provided_root_project_id) = prepared.input.root_project_id
            && provided_root_project_id != canonical_root_project_id
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
                "root_project_id {provided_root_project_id} 与项目 {project_id} 的规范 root {canonical_root_project_id} 不一致"
            )));
        }
        prepared.input.root_project_id = Some(canonical_root_project_id);
    }
    let tx = db.begin().await?;

    require_generation_owner_gates_active(
        &tx,
        prepared.input.requested_by_user_id,
        prepared.input.project_id,
        prepared.input.root_project_id,
    )
    .await?;
    acquire_generation_advisory_lock(
        &tx,
        START_REQUEST_LOCK_NAMESPACE,
        start_request_lock_hash(
            prepared.input.requested_by_user_id,
            prepared.start_request_id,
        ),
    )
    .await?;

    if let Some(mapping) = generation_start_request::Entity::find_by_id((
        prepared.input.requested_by_user_id,
        prepared.start_request_id,
    ))
    .one(&tx)
    .await?
    {
        ensure_start_request_fingerprint(&mapping, &prepared.input_fingerprint_sha256)?;
        touch_start_request_mapping(&tx, mapping.clone()).await?;
        let task = load_persisted_generation_task(&tx, &mapping.task_id).await?;
        let queue_task = load_generation_queue_task(&tx, &mapping.task_id).await?;
        tx.commit().await?;
        return Ok(GenerationTaskReservation {
            created: false,
            task,
            queue_task,
        });
    }

    acquire_generation_advisory_lock(
        &tx,
        BUSINESS_ATTEMPT_LOCK_NAMESPACE,
        business_attempt_lock_hash(&prepared.business_key),
    )
    .await?;

    if let Some(active) = active_generation_task(&tx, &prepared.business_key).await? {
        ensure_active_task_matches(&active, &prepared)?;
        bind_start_request_alias(&tx, &prepared, &active.task_id).await?;
        let queue_task = load_generation_queue_task(&tx, &active.task_id).await?;
        let task = persisted_generation_task_from_checkpoint(active)?;
        tx.commit().await?;
        return Ok(GenerationTaskReservation {
            created: false,
            task,
            queue_task,
        });
    }

    let attempt = next_generation_attempt(&tx, &prepared.business_key).await?;
    validate_retry_lineage(&tx, &prepared, attempt).await?;
    validate_task_tree_lineage(&tx, &prepared).await?;
    validate_related_generation_records(&tx, &prepared.input).await?;

    let task_id = Uuid::new_v4().to_string();
    let now = Utc::now();
    let (checkpoint, public_snapshot) =
        build_reserved_checkpoint(&prepared, task_id.clone(), attempt, now)?;
    insert_generation_queue_record(&tx, &prepared.input, &task_id, now).await?;
    generation_task_checkpoint::ActiveModel::from(checkpoint.clone())
        .insert(&tx)
        .await?;
    insert_start_request_mapping(&tx, &prepared, &task_id, now).await?;
    let queue_task = load_generation_queue_task(&tx, &task_id).await?;
    tx.commit().await?;

    Ok(GenerationTaskReservation {
        created: true,
        task: PersistedGenerationTask {
            checkpoint,
            public_snapshot: Some(public_snapshot),
        },
        queue_task,
    })
}

pub async fn load_generation_task(
    db: &DatabaseConnection,
    task_id: &str,
) -> Result<PersistedGenerationTask, GenerationTaskPersistenceError> {
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    load_persisted_generation_task(db, &task_id).await
}

/// 只读取 checkpoint 规范列；summary/list 需要在 snapshot 正文损坏时继续可用。
pub async fn load_generation_task_checkpoint(
    db: &DatabaseConnection,
    task_id: &str,
) -> Result<generation_task_checkpoint::Model, GenerationTaskPersistenceError> {
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    generation_task_checkpoint::Entity::find_by_id(task_id.clone())
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "generation task checkpoint 不存在: {task_id}"
            ))
        })
}

/// Summary/list 的 snapshot 判定不校验 execution checkpoint，也不因 public snapshot 损坏而丢失规范列。
pub fn generation_public_snapshot_for_summary(
    checkpoint: &generation_task_checkpoint::Model,
) -> GenerationPublicSnapshot {
    match verified_generation_public_snapshot(checkpoint) {
        Ok(Some(snapshot)) => GenerationPublicSnapshot::Available(snapshot),
        Ok(None) => GenerationPublicSnapshot::Expired,
        Err(_) => GenerationPublicSnapshot::Corrupt,
    }
}

pub async fn list_active_generation_task_checkpoints(
    db: &DatabaseConnection,
    requested_by_user_id: i64,
    project_id: Option<i64>,
) -> Result<GenerationTaskCheckpointPage, GenerationTaskPersistenceError> {
    list_generation_task_checkpoints(
        db,
        requested_by_user_id,
        project_id,
        &ACTIVE_GENERATION_STATUSES,
        None,
        MAX_ACTIVE_GENERATION_TASKS,
        false,
    )
    .await
}

pub async fn list_generation_task_history_checkpoints(
    db: &DatabaseConnection,
    requested_by_user_id: i64,
    project_id: Option<i64>,
    limit: u32,
    cursor: Option<GenerationTaskListCursor>,
) -> Result<GenerationTaskCheckpointPage, GenerationTaskPersistenceError> {
    if !(1..=MAX_GENERATION_HISTORY_LIMIT).contains(&limit) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "history limit 必须在 1..={MAX_GENERATION_HISTORY_LIMIT}"
        )));
    }
    list_generation_task_checkpoints(
        db,
        requested_by_user_id,
        project_id,
        &HISTORY_GENERATION_STATUSES,
        cursor,
        limit,
        true,
    )
    .await
}

async fn list_generation_task_checkpoints(
    db: &DatabaseConnection,
    requested_by_user_id: i64,
    project_id: Option<i64>,
    statuses: &[&str],
    cursor: Option<GenerationTaskListCursor>,
    limit: u32,
    exclude_expired: bool,
) -> Result<GenerationTaskCheckpointPage, GenerationTaskPersistenceError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation task list 仅支持 PostgreSQL".to_string(),
        ));
    }
    validate_safe_id(requested_by_user_id, "requested_by_user_id")?;
    if let Some(project_id) = project_id {
        validate_safe_id(project_id, "project_id")?;
    }
    if let Some(cursor) = &cursor {
        validate_safe_cursor(cursor)?;
    }

    let accessible_project_ids =
        crate::collaboration::accessible_project_ids(db, requested_by_user_id)
            .await
            .map_err(|error| match error {
                crate::collaboration::CollaborationDbError::Database(error) => {
                    GenerationTaskPersistenceError::Database(error)
                }
                other => GenerationTaskPersistenceError::Rejected {
                    code: "generation_task_forbidden",
                    message: other.to_string(),
                },
            })?;
    let accessible_projects_placeholder = statuses.len() + 2;

    let mut sql = format!(
        "SELECT checkpoint.task_id\n         FROM generation_task_checkpoint AS checkpoint\n         WHERE checkpoint.requested_by_user_id = $1\n           AND checkpoint.status IN ({})\n           AND EXISTS (\n             SELECT 1\n             FROM \"user\" AS viewer\n             WHERE viewer.id = $1\n               AND viewer.is_active = TRUE\n           )\n           AND (\n             (checkpoint.project_id IS NULL AND checkpoint.root_project_id IS NULL)\n             OR COALESCE(checkpoint.project_id, checkpoint.root_project_id) = ANY(${accessible_projects_placeholder}::BIGINT[])\n           )",
        statuses
            .iter()
            .enumerate()
            .map(|(index, _)| format!("${}", index + 2))
            .collect::<Vec<_>>()
            .join(", ")
    );
    let mut values = Vec::with_capacity(statuses.len() + 5);
    values.push(requested_by_user_id.into());
    for status in statuses {
        values.push((*status).into());
    }
    values.push(accessible_project_ids.into());
    let mut next_placeholder = accessible_projects_placeholder + 1;
    if let Some(project_id) = project_id {
        sql.push_str(&format!(
            "\n           AND (checkpoint.project_id = ${next_placeholder} OR checkpoint.root_project_id = ${next_placeholder})"
        ));
        values.push(project_id.into());
        next_placeholder += 1;
    }
    if exclude_expired {
        sql.push_str(
            "\n           AND (checkpoint.expires_at IS NULL OR checkpoint.expires_at > NOW())",
        );
    }
    if let Some(cursor) = cursor {
        let ms_placeholder = next_placeholder;
        let task_placeholder = next_placeholder + 1;
        sql.push_str(&format!(
            "\n           AND (({GENERATION_UPDATED_AT_MS_SQL} < ${ms_placeholder})\n                OR ({GENERATION_UPDATED_AT_MS_SQL} = ${ms_placeholder} AND checkpoint.task_id > ${task_placeholder}))"
        ));
        values.push(cursor.updated_at_ms.into());
        values.push(cursor.task_id.into());
        next_placeholder += 2;
    }
    let fetch_limit = i64::from(limit) + if exclude_expired { 1 } else { 0 };
    sql.push_str(&format!(
        "\n         ORDER BY {GENERATION_UPDATED_AT_MS_SQL} DESC, checkpoint.task_id ASC\n         LIMIT ${next_placeholder}"
    ));
    values.push(fetch_limit.into());

    let ids = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await?
        .into_iter()
        .map(|row| row.try_get::<String>("", "task_id"))
        .collect::<Result<Vec<_>, _>>()?;
    let has_more = exclude_expired && ids.len() > usize::try_from(limit).unwrap_or(usize::MAX);
    let ids = if has_more {
        &ids[..usize::try_from(limit).unwrap_or(0)]
    } else {
        &ids[..]
    };
    if ids.is_empty() {
        return Ok(GenerationTaskCheckpointPage {
            items: Vec::new(),
            has_more: false,
        });
    }
    let rows = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::TaskId.is_in(ids.iter().cloned()))
        .all(db)
        .await?;
    let mut by_id = rows
        .into_iter()
        .map(|row| (row.task_id.clone(), row))
        .collect::<HashMap<_, _>>();
    let mut items = Vec::with_capacity(ids.len());
    for task_id in ids {
        let Some(row) = by_id.remove(task_id) else {
            return Err(GenerationTaskPersistenceError::Corrupt(format!(
                "generation task checkpoint 查询结果缺少 task_id={task_id}"
            )));
        };
        items.push(row);
    }
    Ok(GenerationTaskCheckpointPage { items, has_more })
}

fn validate_safe_cursor(
    cursor: &GenerationTaskListCursor,
) -> Result<(), GenerationTaskPersistenceError> {
    if !(0..=JS_SAFE_INTEGER_I64).contains(&cursor.updated_at_ms) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "history cursor updated_at_ms 超出 JavaScript safe integer".to_string(),
        ));
    }
    let canonical = canonical_uuid_v4(&cursor.task_id, "history cursor task_id")?;
    if canonical != cursor.task_id {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "history cursor task_id 必须为小写 UUID v4".to_string(),
        ));
    }
    Ok(())
}

pub fn verified_generation_public_snapshot(
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

pub fn verified_generation_execution_checkpoint(
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

pub async fn extend_generation_sequence_lease(
    db: &DatabaseConnection,
    task_id: &str,
    expected_checkpoint_revision: i64,
    expected_reserved_through_sequence: i64,
) -> Result<GenerationSequenceLease, GenerationTaskPersistenceError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation sequence lease 仅支持 PostgreSQL".to_string(),
        ));
    }
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    validate_safe_sequence(expected_checkpoint_revision, "checkpoint_revision")?;
    validate_safe_sequence(
        expected_reserved_through_sequence,
        "reserved_through_sequence",
    )?;
    expected_reserved_through_sequence
        .checked_add(GENERATION_SEQUENCE_LEASE_SIZE)
        .filter(|value| *value <= JS_SAFE_INTEGER_I64)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "reserved_through_sequence 扩租后超过 JavaScript safe integer".to_string(),
            )
        })?;

    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            EXTEND_SEQUENCE_LEASE_SQL,
            [
                task_id.into(),
                expected_checkpoint_revision.into(),
                expected_reserved_through_sequence.into(),
                GENERATION_SEQUENCE_LEASE_SIZE.into(),
                JS_SAFE_INTEGER_I64.into(),
            ],
        ))
        .await?
    else {
        return Err(GenerationTaskPersistenceError::LeaseConflict);
    };
    Ok(GenerationSequenceLease {
        checkpoint_revision: row.try_get("", "checkpoint_revision")?,
        reserved_through_sequence: row.try_get("", "reserved_through_sequence")?,
    })
}

pub async fn persist_generation_checkpoint(
    db: &DatabaseConnection,
    input: PersistGenerationCheckpointInput,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskPersistenceError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation checkpoint persistence 仅支持 PostgreSQL".to_string(),
        ));
    }
    let task_id = canonical_uuid_v4(&input.task_id, "task_id")?;
    if input.public_snapshot.task_id != task_id || input.public_snapshot.attempt != input.attempt {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "public snapshot task_id/attempt 与 checkpoint input 不一致".to_string(),
        ));
    }
    if !matches!(
        input.public_snapshot.status,
        GenerationTaskStatus::Reserved
            | GenerationTaskStatus::Running
            | GenerationTaskStatus::Cancelling
    ) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "普通 checkpoint 只允许 reserved/running/cancelling".to_string(),
        ));
    }
    input
        .public_snapshot
        .validate()
        .map_err(|error| GenerationTaskPersistenceError::InvalidArgument(error.to_string()))?;
    validate_safe_sequence(
        input.expected_checkpoint_revision,
        "expected_checkpoint_revision",
    )?;
    let attempt = i32::try_from(input.attempt).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument("attempt 超出 i32 范围".to_string())
    })?;
    let covers_through_sequence = i64::try_from(input.public_snapshot.covers_through_sequence)
        .map_err(|_| {
            GenerationTaskPersistenceError::InvalidArgument(
                "covers_through_sequence 超出 i64 范围".to_string(),
            )
        })?;
    validate_safe_sequence(covers_through_sequence, "covers_through_sequence")?;
    let status = enum_storage(input.public_snapshot.status, "status")?;
    let public_snapshot_json = serde_json::to_value(&input.public_snapshot)?;
    let public_snapshot_sha256 = canonical_json_sha256(&public_snapshot_json)?;
    let step_key = normalize_optional_text(input.step_key, "step_key", 256)?;
    let call_key = normalize_optional_text(input.call_key, "call_key", 256)?;
    let provider_kind = normalize_optional_text(input.provider_kind, "provider_kind", 128)?;
    let provider_model = normalize_optional_text(input.provider_model, "provider_model", 256)?;
    let provider_job_id = normalize_optional_text(input.provider_job_id, "provider_job_id", 256)?;
    let provider_idempotency_key_sha256 = validate_optional_sha256(
        input.provider_idempotency_key_sha256,
        "provider_idempotency_key_sha256",
    )?;
    if provider_job_id.is_some()
        && (provider_kind.is_none() || provider_idempotency_key_sha256.is_none())
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "provider_job_id 必须与 provider_kind/idempotency hash 一起写入".to_string(),
        ));
    }
    let (resume_metadata_version, execution_checkpoint_json, execution_checkpoint_sha256) = match (
        input.resume_metadata_version,
        input.execution_checkpoint_json,
    ) {
        (None, None) => (None, None, None),
        (Some(version), Some(checkpoint)) if version > 0 && provider_kind.is_some() => {
            let sha256 = canonical_json_sha256(&checkpoint)?;
            (Some(version), Some(checkpoint), Some(sha256))
        }
        _ => {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                    "execution checkpoint JSON、正数 resume_metadata_version 与 provider_kind 必须一起写入"
                        .to_string(),
                ));
        }
    };

    let Some(row) = db
        .query_one(Statement::from_sql_and_values(
            DbBackend::Postgres,
            PERSIST_CHECKPOINT_SQL,
            [
                task_id.into(),
                attempt.into(),
                input.expected_checkpoint_revision.into(),
                status.into(),
                public_snapshot_json.into(),
                public_snapshot_sha256.into(),
                covers_through_sequence.into(),
                step_key.into(),
                call_key.into(),
                provider_kind.into(),
                provider_model.into(),
                provider_job_id.into(),
                provider_idempotency_key_sha256.into(),
                resume_metadata_version.into(),
                execution_checkpoint_json.into(),
                execution_checkpoint_sha256.into(),
                JS_SAFE_INTEGER_I64.into(),
            ],
        ))
        .await?
    else {
        return Err(GenerationTaskPersistenceError::CheckpointConflict);
    };
    Ok(PersistedGenerationCheckpoint {
        checkpoint_revision: row.try_get("", "checkpoint_revision")?,
        covers_through_sequence: row.try_get("", "covers_through_sequence")?,
        reserved_through_sequence: row.try_get("", "reserved_through_sequence")?,
        updated_at: row.try_get("", "updated_at")?,
    })
}

fn prepare_reservation(
    mut input: ReserveGenerationTaskInput,
) -> Result<PreparedReservation, GenerationTaskPersistenceError> {
    validate_safe_id(input.requested_by_user_id, "requested_by_user_id")?;
    let start_request_id = parse_uuid_v4(&input.start_request_id, "start_request_id")?;
    let business_key_object = input.business_key_json.as_object().ok_or_else(|| {
        GenerationTaskPersistenceError::InvalidArgument(
            "business_key_json 必须是含 version 的对象".to_string(),
        )
    })?;
    if business_key_object
        .get("version")
        .is_none_or(serde_json::Value::is_null)
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "business_key_json 缺少非空 version".to_string(),
        ));
    }
    if !input.input_fingerprint_json.is_object() {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "input_fingerprint_json 必须是对象".to_string(),
        ));
    }
    let business_key = canonical_json_text(&input.business_key_json)?;
    if !(2..=2048).contains(&business_key.len()) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "business_key canonical JSON 必须在 2..=2048 字节内".to_string(),
        ));
    }
    let input_fingerprint_sha256 = canonical_json_sha256(&input.input_fingerprint_json)?;

    validate_optional_safe_id(input.project_id, "project_id")?;
    validate_optional_safe_id(input.root_project_id, "root_project_id")?;
    validate_optional_safe_id(input.source_snapshot_id, "source_snapshot_id")?;
    validate_optional_safe_id(input.workflow_run_id, "workflow_run_id")?;
    validate_optional_safe_id(input.generation_run_id, "generation_run_id")?;
    if input.project_id.is_none() && input.root_project_id.is_some() {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "无 project_id 时不能设置 root_project_id".to_string(),
        ));
    }
    input.task_scope = normalize_optional_text(input.task_scope, "task_scope", 128)?;
    input.root_task_id = normalize_optional_uuid(input.root_task_id, "root_task_id")?;
    input.parent_task_id = normalize_optional_uuid(input.parent_task_id, "parent_task_id")?;
    input.retry_of_task_id = normalize_optional_uuid(input.retry_of_task_id, "retry_of_task_id")?;
    input.item_key = normalize_optional_text(input.item_key, "item_key", 256)?;
    match (
        input.root_task_id.is_some(),
        input.parent_task_id.is_some(),
        input.item_key.is_some(),
    ) {
        (false, false, false) | (true, true, true) => {}
        _ => {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "child task 必须同时提供 root_task_id、parent_task_id 和 item_key".to_string(),
            ));
        }
    }
    input.target = normalize_target(input.target)?;

    Ok(PreparedReservation {
        input,
        start_request_id,
        business_key,
        input_fingerprint_sha256,
    })
}

fn normalize_target(
    target: Option<GenerationTargetRef>,
) -> Result<Option<GenerationTargetRef>, GenerationTaskPersistenceError> {
    target
        .map(|target| {
            Ok(GenerationTargetRef {
                kind: required_text(target.kind, "target.kind", 128)?,
                id: required_text(target.id, "target.id", 256)?,
                revision: normalize_optional_text(target.revision, "target.revision", 256)?,
            })
        })
        .transpose()
}

async fn acquire_generation_advisory_lock(
    tx: &DatabaseTransaction,
    namespace: i32,
    key: i32,
) -> Result<(), GenerationTaskPersistenceError> {
    tx.execute(Statement::from_sql_and_values(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock($1, $2)",
        [namespace.into(), key.into()],
    ))
    .await?;
    Ok(())
}

fn start_request_lock_hash(requested_by_user_id: i64, start_request_id: Uuid) -> i32 {
    let mut hasher = Sha256::new();
    hasher.update(requested_by_user_id.to_be_bytes());
    hasher.update([0]);
    hasher.update(start_request_id.as_bytes());
    digest_prefix_i32(hasher.finalize())
}

fn business_attempt_lock_hash(business_key: &str) -> i32 {
    digest_prefix_i32(Sha256::digest(business_key.as_bytes()))
}

fn digest_prefix_i32(digest: impl AsRef<[u8]>) -> i32 {
    let digest = digest.as_ref();
    i32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]])
}

fn ensure_start_request_fingerprint(
    mapping: &generation_start_request::Model,
    expected_sha256: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    if mapping.input_fingerprint_sha256 == expected_sha256 {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_start_request_conflict",
            message: "相同 start_request_id 对应了不同输入".to_string(),
        })
    }
}

async fn touch_start_request_mapping(
    tx: &DatabaseTransaction,
    mapping: generation_start_request::Model,
) -> Result<(), GenerationTaskPersistenceError> {
    let mut active: generation_start_request::ActiveModel = mapping.into();
    active.last_seen_at = Set(Utc::now().naive_utc());
    active.update(tx).await?;
    Ok(())
}

async fn active_generation_task(
    tx: &DatabaseTransaction,
    business_key: &str,
) -> Result<Option<generation_task_checkpoint::Model>, GenerationTaskPersistenceError> {
    Ok(generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::BusinessKey.eq(business_key))
        .filter(generation_task_checkpoint::Column::Status.is_in(ACTIVE_GENERATION_STATUSES))
        .lock_exclusive()
        .one(tx)
        .await?)
}

fn ensure_active_task_matches(
    active: &generation_task_checkpoint::Model,
    prepared: &PreparedReservation,
) -> Result<(), GenerationTaskPersistenceError> {
    let desired_is_root = prepared.input.root_task_id.is_none();
    let active_is_root = active.root_task_id == active.task_id
        && active.parent_task_id.is_none()
        && active.item_key.is_none();
    let target_matches = match (
        &prepared.input.target,
        &active.target_kind,
        &active.target_id,
    ) {
        (None, None, None) => active.target_revision.is_none(),
        (Some(target), Some(kind), Some(id)) => {
            target.kind.as_str() == kind.as_str()
                && target.id.as_str() == id.as_str()
                && target.revision.as_deref() == active.target_revision.as_deref()
        }
        _ => false,
    };
    let batch_policy = prepared
        .input
        .batch_policy
        .map(|policy| enum_storage(policy, "batch_policy"))
        .transpose()?;
    let delivery_mode = enum_storage(prepared.input.delivery_mode, "delivery_mode")?;
    let lineage_matches = if desired_is_root {
        active_is_root
    } else {
        active.root_task_id == prepared.input.root_task_id.as_deref().unwrap_or_default()
            && active.parent_task_id.as_deref() == prepared.input.parent_task_id.as_deref()
            && active.item_key.as_deref() == prepared.input.item_key.as_deref()
    };
    if active.requested_by_user_id == prepared.input.requested_by_user_id
        && active.input_fingerprint_sha256 == prepared.input_fingerprint_sha256
        && active.task_kind == prepared.input.task_kind.as_str()
        && active.delivery_mode == delivery_mode
        && active.batch_policy.as_deref() == batch_policy.as_deref()
        && active.project_id == prepared.input.project_id
        && active.root_project_id == prepared.input.root_project_id
        && active.workflow_run_id == prepared.input.workflow_run_id
        && active.generation_run_id == prepared.input.generation_run_id
        && active.retry_of_task_id.as_deref() == prepared.input.retry_of_task_id.as_deref()
        && target_matches
        && lineage_matches
    {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_attempt_conflict",
            message: "相同 business key 的 active task 与本次请求不一致".to_string(),
        })
    }
}

async fn bind_start_request_alias(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
    task_id: &str,
) -> Result<(), GenerationTaskPersistenceError> {
    let mapping_count = generation_start_request::Entity::find()
        .filter(generation_start_request::Column::TaskId.eq(task_id))
        .count(tx)
        .await?;
    if mapping_count >= MAX_GENERATION_START_REQUEST_MAPPINGS {
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_start_request_capacity_exceeded",
            message: format!(
                "任务 {task_id} 已达到 {MAX_GENERATION_START_REQUEST_MAPPINGS} 个 start request 映射上限"
            ),
        });
    }
    insert_start_request_mapping(tx, prepared, task_id, Utc::now()).await
}

async fn next_generation_attempt(
    tx: &DatabaseTransaction,
    business_key: &str,
) -> Result<i32, GenerationTaskPersistenceError> {
    let previous = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::BusinessKey.eq(business_key))
        .order_by_desc(generation_task_checkpoint::Column::Attempt)
        .one(tx)
        .await?;
    previous
        .map(|checkpoint| checkpoint.attempt)
        .unwrap_or(0)
        .checked_add(1)
        .filter(|attempt| *attempt > 0)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "generation attempt 已耗尽 i32 范围".to_string(),
            )
        })
}

async fn validate_retry_lineage(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
    new_attempt: i32,
) -> Result<(), GenerationTaskPersistenceError> {
    let Some(retry_of_task_id) = prepared.input.retry_of_task_id.as_deref() else {
        return Ok(());
    };
    let retry_of = generation_task_checkpoint::Entity::find_by_id(retry_of_task_id.to_string())
        .lock_shared()
        .one(tx)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "retry_of_task_id 不存在: {retry_of_task_id}"
            ))
        })?;
    let retry_is_root = retry_of.root_task_id == retry_of.task_id
        && retry_of.parent_task_id.is_none()
        && retry_of.item_key.is_none();
    let requested_tree_matches = if retry_is_root {
        prepared.input.root_task_id.is_none()
            && prepared.input.parent_task_id.is_none()
            && prepared.input.item_key.is_none()
    } else {
        prepared.input.root_task_id.as_deref() == Some(retry_of.root_task_id.as_str())
            && prepared.input.parent_task_id.as_deref() == retry_of.parent_task_id.as_deref()
            && prepared.input.item_key.as_deref() == retry_of.item_key.as_deref()
    };
    if retry_of.business_key != prepared.business_key
        || retry_of.requested_by_user_id != prepared.input.requested_by_user_id
        || retry_of.project_id != prepared.input.project_id
        || retry_of.root_project_id != prepared.input.root_project_id
        || retry_of.attempt >= new_attempt
        || !requested_tree_matches
    {
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_attempt_conflict",
            message: "retry_of_task_id 与当前 business/user/project/task tree 不一致".to_string(),
        });
    }
    Ok(())
}

async fn validate_task_tree_lineage(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
) -> Result<(), GenerationTaskPersistenceError> {
    let (Some(root_task_id), Some(parent_task_id)) = (
        prepared.input.root_task_id.as_deref(),
        prepared.input.parent_task_id.as_deref(),
    ) else {
        return Ok(());
    };
    let root = generation_task_checkpoint::Entity::find_by_id(root_task_id.to_string())
        .lock_shared()
        .one(tx)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!("root_task_id 不存在: {root_task_id}"))
        })?;
    let parent = generation_task_checkpoint::Entity::find_by_id(parent_task_id.to_string())
        .lock_shared()
        .one(tx)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "parent_task_id 不存在: {parent_task_id}"
            ))
        })?;
    if root.root_task_id != root.task_id
        || parent.root_task_id != root.task_id
        || root.requested_by_user_id != prepared.input.requested_by_user_id
        || parent.requested_by_user_id != prepared.input.requested_by_user_id
        || root.project_id != prepared.input.project_id
        || parent.project_id != prepared.input.project_id
        || root.root_project_id != prepared.input.root_project_id
        || parent.root_project_id != prepared.input.root_project_id
    {
        return Err(GenerationTaskPersistenceError::Rejected {
            code: "generation_attempt_conflict",
            message: "root/parent task 与当前 owner/project task tree 不一致".to_string(),
        });
    }
    Ok(())
}

async fn validate_related_generation_records(
    tx: &DatabaseTransaction,
    input: &ReserveGenerationTaskInput,
) -> Result<(), GenerationTaskPersistenceError> {
    if let Some(workflow_run_id) = input.workflow_run_id {
        let run = workflow_run::Entity::find_by_id(workflow_run_id)
            .lock_shared()
            .one(tx)
            .await?
            .ok_or_else(|| {
                GenerationTaskPersistenceError::NotFound(format!(
                    "workflow_run 不存在: {workflow_run_id}"
                ))
            })?;
        if run.project_id != input.project_id
            || run.root_project_id.or(run.project_id) != input.root_project_id
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "workflow_run 与 generation task project 归属不一致".to_string(),
            ));
        }
    }
    if let Some(generation_run_id) = input.generation_run_id {
        let run = generation_run::Entity::find_by_id(generation_run_id)
            .lock_shared()
            .one(tx)
            .await?
            .ok_or_else(|| {
                GenerationTaskPersistenceError::NotFound(format!(
                    "generation_run 不存在: {generation_run_id}"
                ))
            })?;
        if input.project_id != Some(run.project_id)
            || input.root_project_id != Some(run.root_project_id)
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "generation_run 与 generation task project 归属不一致".to_string(),
            ));
        }
    }
    Ok(())
}

fn build_reserved_checkpoint(
    prepared: &PreparedReservation,
    task_id: String,
    attempt: i32,
    now: DateTime<Utc>,
) -> Result<
    (generation_task_checkpoint::Model, GenerationTaskSnapshot),
    GenerationTaskPersistenceError,
> {
    let attempt_u32 = u32::try_from(attempt).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument("attempt 超出 u32 范围".to_string())
    })?;
    let root_task_id = prepared
        .input
        .root_task_id
        .clone()
        .unwrap_or_else(|| task_id.clone());
    let started = StartedEvent {
        task_kind: prepared.input.task_kind.as_str().to_string(),
        delivery_mode: prepared.input.delivery_mode,
        project_id: prepared.input.project_id,
        root_project_id: prepared.input.root_project_id,
        workflow_run_id: prepared.input.workflow_run_id,
        generation_run_id: prepared.input.generation_run_id,
        target: prepared.input.target.clone(),
        batch_policy: prepared.input.batch_policy,
        root_task_id: root_task_id.clone(),
        parent_task_id: prepared.input.parent_task_id.clone(),
        item_key: prepared.input.item_key.clone(),
        retry_of_task_id: prepared.input.retry_of_task_id.clone(),
    };
    let public_snapshot = GenerationTaskSnapshot {
        schema_version: GENERATION_EVENT_SCHEMA_VERSION,
        task_id: task_id.clone(),
        attempt: attempt_u32,
        covers_through_sequence: 1,
        status: GenerationTaskStatus::Reserved,
        started: started.clone(),
        cancel_reason: None,
        progress: None,
        streams: Vec::new(),
        artifacts: Vec::new(),
        usage: None,
        latest_checkpoint: None,
        terminal_event: None,
        updated_at_ms: now.timestamp_millis(),
    };
    public_snapshot
        .validate()
        .map_err(|error| GenerationTaskPersistenceError::InvalidArgument(error.to_string()))?;
    let public_snapshot_json = serde_json::to_value(&public_snapshot)?;
    let public_snapshot_sha256 = canonical_json_sha256(&public_snapshot_json)?;
    let target = prepared.input.target.as_ref();
    let now_naive = now.naive_utc();
    let checkpoint = generation_task_checkpoint::Model {
        task_id,
        business_key: prepared.business_key.clone(),
        attempt,
        requested_by_user_id: prepared.input.requested_by_user_id,
        task_kind: prepared.input.task_kind.as_str().to_string(),
        delivery_mode: enum_storage(prepared.input.delivery_mode, "delivery_mode")?,
        batch_policy: prepared
            .input
            .batch_policy
            .map(|policy| enum_storage(policy, "batch_policy"))
            .transpose()?,
        project_id: prepared.input.project_id,
        root_project_id: prepared.input.root_project_id,
        workflow_run_id: prepared.input.workflow_run_id,
        generation_run_id: prepared.input.generation_run_id,
        target_kind: target.map(|target| target.kind.clone()),
        target_id: target.map(|target| target.id.clone()),
        target_revision: target.and_then(|target| target.revision.clone()),
        root_task_id,
        parent_task_id: prepared.input.parent_task_id.clone(),
        item_key: prepared.input.item_key.clone(),
        retry_of_task_id: prepared.input.retry_of_task_id.clone(),
        status: enum_storage(GenerationTaskStatus::Reserved, "status")?,
        checkpoint_revision: 1,
        covers_through_sequence: 1,
        reserved_through_sequence: INITIAL_RESERVED_THROUGH_SEQUENCE,
        public_snapshot_json: Some(public_snapshot_json),
        public_snapshot_sha256: Some(public_snapshot_sha256),
        step_key: None,
        call_key: None,
        provider_kind: None,
        provider_model: None,
        provider_job_id: None,
        provider_idempotency_key_sha256: None,
        resume_metadata_version: None,
        execution_checkpoint_json: None,
        execution_checkpoint_sha256: None,
        input_fingerprint_sha256: prepared.input_fingerprint_sha256.clone(),
        result_kind: None,
        result_id: None,
        result_revision: None,
        expires_at: None,
        created_at: now_naive,
        updated_at: now_naive,
        completed_at: None,
    };
    Ok((checkpoint, public_snapshot))
}

async fn insert_generation_queue_record(
    tx: &DatabaseTransaction,
    input: &ReserveGenerationTaskInput,
    task_id: &str,
    now: DateTime<Utc>,
) -> Result<(), GenerationTaskPersistenceError> {
    task_queue::ActiveModel {
        id: Set(task_id.to_string()),
        task_type: Set(input.task_kind.as_str().to_string()),
        priority: Set(input.priority.as_str().to_string()),
        status: Set("pending".to_string()),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        task_scope: Set(input.task_scope.clone()),
        workflow_run_id: Set(input.workflow_run_id),
        payload: Set(None),
        progress: Set(None),
        error_message: Set(None),
        dag_id: Set(None),
        created_at: Set(now.naive_utc()),
        started_at: Set(None),
        completed_at: Set(None),
    }
    .insert(tx)
    .await?;
    Ok(())
}

async fn insert_start_request_mapping(
    tx: &DatabaseTransaction,
    prepared: &PreparedReservation,
    task_id: &str,
    now: DateTime<Utc>,
) -> Result<(), GenerationTaskPersistenceError> {
    let now = now.naive_utc();
    generation_start_request::ActiveModel {
        requested_by_user_id: Set(prepared.input.requested_by_user_id),
        start_request_id: Set(prepared.start_request_id),
        task_id: Set(task_id.to_string()),
        input_fingerprint_sha256: Set(prepared.input_fingerprint_sha256.clone()),
        created_at: Set(now),
        last_seen_at: Set(now),
    }
    .insert(tx)
    .await?;
    Ok(())
}

async fn load_persisted_generation_task<C>(
    db: &C,
    task_id: &str,
) -> Result<PersistedGenerationTask, GenerationTaskPersistenceError>
where
    C: ConnectionTrait,
{
    let checkpoint = generation_task_checkpoint::Entity::find_by_id(task_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "generation task checkpoint 不存在: {task_id}"
            ))
        })?;
    persisted_generation_task_from_checkpoint(checkpoint)
}

async fn load_generation_queue_task<C>(
    db: &C,
    task_id: &str,
) -> Result<TaskInfo, GenerationTaskPersistenceError>
where
    C: ConnectionTrait,
{
    let row = task_queue::Entity::find_by_id(task_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::Corrupt(format!(
                "generation task {task_id} 缺少 task_queue row"
            ))
        })?;
    crate::workflow::task_info_from_row(row)
        .map_err(|error| GenerationTaskPersistenceError::Corrupt(error.to_string()))
}

pub fn persisted_generation_task_from_checkpoint(
    checkpoint: generation_task_checkpoint::Model,
) -> Result<PersistedGenerationTask, GenerationTaskPersistenceError> {
    let public_snapshot = verified_generation_public_snapshot(&checkpoint)?;
    verified_generation_execution_checkpoint(&checkpoint)?;
    Ok(PersistedGenerationTask {
        checkpoint,
        public_snapshot,
    })
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

pub fn started_event_from_checkpoint(
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
        let prepared = prepare_reservation(reservation_input()).expect("prepared reservation");
        let now = Utc.timestamp_millis_opt(1_750_000_000_000).unwrap();
        let (checkpoint, snapshot) = build_reserved_checkpoint(
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
        assert_eq!(start_request_lock_hash(7, request_id), 1_496_059_712);
        assert_eq!(
            business_attempt_lock_hash("{\"project_id\":11,\"version\":1}"),
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
