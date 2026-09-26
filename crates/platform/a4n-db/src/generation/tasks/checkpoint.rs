use super::*;
use a4n_db_models::models::task::task_queue;
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

/// Binds a reserved generation task to the workflow created by its worker.
///
/// Generation reservations happen before the worker can create its workflow;
/// keep the normalized checkpoint columns and public snapshot in sync once the
/// workflow claim succeeds.
pub async fn bind_generation_task_to_workflow(
    tx: &DatabaseTransaction,
    task_id: &str,
    workflow_run_id: i64,
) -> Result<(), GenerationTaskPersistenceError> {
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    validate_safe_id(workflow_run_id, "workflow_run_id")?;
    if let Some(task) = task_queue::Entity::find_by_id(task_id.clone())
        .lock_exclusive()
        .one(tx)
        .await?
    {
        crate::queue::bind_locked_task_queue_row_to_workflow(tx, task, workflow_run_id)
            .await
            .map_err(|error| match error {
                crate::queue::TaskWorkflowBindingError::Conflict { current } => {
                    GenerationTaskPersistenceError::Corrupt(format!(
                        "generation task {task_id} 的 task_queue 已绑定 workflow {current}"
                    ))
                }
                crate::queue::TaskWorkflowBindingError::Database(error) => error.into(),
            })?;
    }
    let Some(checkpoint) = generation_task_checkpoint::Entity::find_by_id(task_id.clone())
        .lock_exclusive()
        .one(tx)
        .await?
    else {
        return Ok(());
    };
    match checkpoint.workflow_run_id {
        Some(current) if current == workflow_run_id => return Ok(()),
        Some(current) => {
            let previous = workflow_run::Entity::find_by_id(current).one(tx).await?;
            if previous.is_none_or(|run| run.status != WORKFLOW_STATUS_COMPLETED) {
                return Err(GenerationTaskPersistenceError::Corrupt(format!(
                    "generation task {task_id} 已绑定未完成 workflow {current}，不能改绑 {workflow_run_id}"
                )));
            }
        }
        None => {}
    }
    let snapshot_json = checkpoint.public_snapshot_json.as_ref().ok_or_else(|| {
        GenerationTaskPersistenceError::Corrupt(format!(
            "generation task {task_id} 缺少 public snapshot，不能绑定 workflow"
        ))
    })?;
    let mut snapshot: GenerationTaskSnapshot = serde_json::from_value(snapshot_json.clone())?;
    snapshot.started.workflow_run_id = Some(workflow_run_id);
    snapshot
        .validate()
        .map_err(|error| GenerationTaskPersistenceError::Corrupt(error.to_string()))?;
    let public_snapshot_json = serde_json::to_value(&snapshot)?;
    let public_snapshot_sha256 = canonical_json_sha256(&public_snapshot_json)?;
    let checkpoint_revision = checkpoint
        .checkpoint_revision
        .checked_add(1)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::Corrupt(format!(
                "generation task {task_id} checkpoint_revision 超出范围"
            ))
        })?;
    let mut update: generation_task_checkpoint::ActiveModel = checkpoint.into();
    update.workflow_run_id = Set(Some(workflow_run_id));
    update.public_snapshot_json = Set(Some(public_snapshot_json));
    update.public_snapshot_sha256 = Set(Some(public_snapshot_sha256));
    update.checkpoint_revision = Set(checkpoint_revision);
    update.updated_at = Set(Utc::now().naive_utc());
    update.update(tx).await?;
    Ok(())
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

pub(super) async fn load_persisted_generation_task<C>(
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

pub(super) async fn load_generation_queue_task<C>(
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
