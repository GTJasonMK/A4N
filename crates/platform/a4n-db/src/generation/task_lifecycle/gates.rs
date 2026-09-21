use super::*;
pub async fn persist_generation_status_changed(
    db: &DatabaseConnection,
    input: PersistGenerationLifecycleInput,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError> {
    validate_transition_input(db, &input, false)?;
    if !valid_status_changed_transition(input.expected_status, input.public_snapshot.status) {
        return Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "非法 generation status_changed transition: {:?}→{:?}",
            input.expected_status, input.public_snapshot.status
        ))
        .into());
    }
    persist_transition(db, input, false, false, true).await
}

pub(super) fn valid_status_changed_transition(
    previous: GenerationTaskStatus,
    next: GenerationTaskStatus,
) -> bool {
    matches!(
        (previous, next),
        (
            GenerationTaskStatus::Reserved,
            GenerationTaskStatus::Running
        ) | (
            GenerationTaskStatus::Running,
            GenerationTaskStatus::Finalizing
        )
    )
}

pub async fn persist_generation_terminal(
    db: &DatabaseConnection,
    input: PersistGenerationLifecycleInput,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError> {
    validate_transition_input(db, &input, true)?;
    validate_terminal_transition(input.expected_status, input.public_snapshot.status)?;
    persist_transition(db, input, true, false, false).await
}

/// completed 事务内已取得的 generation checkpoint 锁；字段仅由 DB application 持有。
pub struct LockedGenerationCompletion {
    checkpoint: generation_task_checkpoint::Model,
}

pub async fn lock_generation_completion(
    tx: &DatabaseTransaction,
    task_id: &str,
    attempt: u32,
    expected_checkpoint_revision: i64,
) -> Result<LockedGenerationCompletion, GenerationTaskLifecycleError> {
    if tx.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation lifecycle persistence 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    validate_safe_sequence(expected_checkpoint_revision, "expected_checkpoint_revision")?;
    let checkpoint = generation_task_checkpoint::Entity::find_by_id(task_id.clone())
        .lock_exclusive()
        .one(tx)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "generation task checkpoint 不存在: {task_id}"
            ))
        })?;
    let attempt = i32::try_from(attempt).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument("attempt 超出 i32 范围".to_string())
    })?;
    if checkpoint.attempt != attempt
        || checkpoint.checkpoint_revision != expected_checkpoint_revision
        || checkpoint.status != "finalizing"
    {
        return Err(GenerationTaskLifecycleError::Conflict {
            current_status: checkpoint.status,
            current_revision: checkpoint.checkpoint_revision,
        });
    }
    Ok(LockedGenerationCompletion { checkpoint })
}

pub async fn persist_locked_generation_completion(
    tx: &DatabaseTransaction,
    locked: LockedGenerationCompletion,
    input: PersistGenerationLifecycleInput,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError> {
    validate_transition_input(tx, &input, true)?;
    if input.expected_status != GenerationTaskStatus::Finalizing
        || input.public_snapshot.status != GenerationTaskStatus::Completed
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "completed transaction 只允许 finalizing→completed".to_string(),
        )
        .into());
    }
    let prepared = prepare_transition_from_checkpoint(locked.checkpoint, input, true, false)?;
    apply_prepared_transition(tx, prepared).await
}

pub async fn load_generation_corrupt_recovery_state(
    db: &DatabaseConnection,
    task_id: &str,
) -> Result<GenerationCorruptRecoveryState, GenerationTaskLifecycleError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation corrupt recovery 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    let checkpoint = generation_task_checkpoint::Entity::find_by_id(task_id.clone())
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "generation task checkpoint 不存在: {task_id}"
            ))
        })?;
    let attempt = u32::try_from(checkpoint.attempt).map_err(|_| {
        GenerationTaskLifecycleError::Corrupt(format!("generation task {task_id} attempt 为负数"))
    })?;
    let expected_status: GenerationTaskStatus = enum_from_storage(&checkpoint.status, "status")?;
    if expected_status.is_terminal() {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {task_id} 已是 terminal，不能执行 corrupt recovery"
        )));
    }
    let reserved_through_sequence =
        u64::try_from(checkpoint.reserved_through_sequence).map_err(|_| {
            GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {task_id} sequence high-water 为负数"
            ))
        })?;
    let batch_summary = corrupt_recovery_batch_summary(db, &checkpoint).await?;
    Ok(GenerationCorruptRecoveryState {
        task_id,
        attempt,
        expected_checkpoint_revision: checkpoint.checkpoint_revision,
        expected_status,
        reserved_through_sequence,
        started: started_event_from_checkpoint(&checkpoint)?,
        batch_summary,
    })
}

pub async fn persist_generation_corrupt_terminal(
    db: &DatabaseConnection,
    input: PersistGenerationLifecycleInput,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError> {
    validate_transition_input(db, &input, true)?;
    if input.public_snapshot.status != GenerationTaskStatus::Failed {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "corrupt recovery 只允许写 failed terminal".to_string(),
        )
        .into());
    }
    validate_terminal_transition(input.expected_status, input.public_snapshot.status)?;
    persist_transition(db, input, true, true, false).await
}

pub(super) fn validate_transition_input<C>(
    db: &C,
    input: &PersistGenerationLifecycleInput,
    terminal: bool,
) -> Result<(), GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation lifecycle persistence 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    let task_id = canonical_uuid_v4(&input.task_id, "task_id")?;
    if task_id != input.public_snapshot.task_id || input.public_snapshot.attempt != input.attempt {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "lifecycle snapshot task_id/attempt 与 input 不一致".to_string(),
        )
        .into());
    }
    validate_safe_sequence(
        input.expected_checkpoint_revision,
        "expected_checkpoint_revision",
    )?;
    input
        .public_snapshot
        .validate()
        .map_err(|error| GenerationTaskPersistenceError::InvalidArgument(error.to_string()))?;
    if input.expected_status.is_terminal()
        || input.public_snapshot.status.is_terminal() != terminal
        || (!terminal && input.public_snapshot.terminal_event.is_some())
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation lifecycle transition 状态与 snapshot 不一致".to_string(),
        )
        .into());
    }
    Ok(())
}

pub(super) fn validate_terminal_transition(
    previous: GenerationTaskStatus,
    terminal: GenerationTaskStatus,
) -> Result<(), GenerationTaskLifecycleError> {
    let valid = match terminal {
        GenerationTaskStatus::Completed => previous == GenerationTaskStatus::Finalizing,
        GenerationTaskStatus::Cancelled => previous == GenerationTaskStatus::Cancelling,
        GenerationTaskStatus::Failed | GenerationTaskStatus::Interrupted => !previous.is_terminal(),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(GenerationTaskPersistenceError::InvalidArgument(format!(
            "非法 generation terminal transition: {previous:?}→{terminal:?}"
        ))
        .into())
    }
}

pub(super) async fn persist_transition(
    db: &DatabaseConnection,
    input: PersistGenerationLifecycleInput,
    terminal: bool,
    allow_corrupt_previous_snapshot: bool,
    require_active_owner_gates: bool,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError> {
    let tx = db.begin().await?;
    let prepared = prepare_transition(
        &tx,
        input,
        terminal,
        allow_corrupt_previous_snapshot,
        require_active_owner_gates,
    )
    .await?;
    let persisted = apply_prepared_transition(&tx, prepared).await?;
    tx.commit().await?;
    Ok(persisted)
}

pub(super) struct PreparedGenerationTransition {
    pub(super) checkpoint: generation_task_checkpoint::Model,
    pub(super) input: PersistGenerationLifecycleInput,
    pub(super) terminal: bool,
}

pub(super) async fn prepare_transition<C>(
    db: &C,
    input: PersistGenerationLifecycleInput,
    terminal: bool,
    allow_corrupt_previous_snapshot: bool,
    require_active_owner_gates: bool,
) -> Result<PreparedGenerationTransition, GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    if require_active_owner_gates {
        let owner = generation_task_checkpoint::Entity::find_by_id(input.task_id.clone())
            .one(db)
            .await?
            .ok_or_else(|| {
                GenerationTaskPersistenceError::NotFound(format!(
                    "generation task checkpoint 不存在: {}",
                    input.task_id
                ))
            })?;
        require_generation_owner_gates_active(
            db,
            owner.requested_by_user_id,
            owner.project_id,
            owner.root_project_id,
        )
        .await?;
    }
    let checkpoint = generation_task_checkpoint::Entity::find_by_id(input.task_id.clone())
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "generation task checkpoint 不存在: {}",
                input.task_id
            ))
        })?;
    prepare_transition_from_checkpoint(checkpoint, input, terminal, allow_corrupt_previous_snapshot)
}

pub(super) fn prepare_transition_from_checkpoint(
    checkpoint: generation_task_checkpoint::Model,
    input: PersistGenerationLifecycleInput,
    terminal: bool,
    allow_corrupt_previous_snapshot: bool,
) -> Result<PreparedGenerationTransition, GenerationTaskLifecycleError> {
    let expected_status = enum_storage(input.expected_status, "expected_status")?;
    let attempt = i32::try_from(input.attempt).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument("attempt 超出 i32 范围".to_string())
    })?;
    if checkpoint.attempt != attempt
        || checkpoint.checkpoint_revision != input.expected_checkpoint_revision
        || checkpoint.status != expected_status
    {
        return Err(GenerationTaskLifecycleError::Conflict {
            current_status: checkpoint.status,
            current_revision: checkpoint.checkpoint_revision,
        });
    }
    let previous_covers = if allow_corrupt_previous_snapshot {
        if started_event_from_checkpoint(&checkpoint)? != input.public_snapshot.started {
            return Err(GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {} corrupt recovery started 与规范列不一致",
                input.task_id
            )));
        }
        u64::try_from(checkpoint.covers_through_sequence).map_err(|_| {
            GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {} covers 为负数",
                input.task_id
            ))
        })?
    } else {
        let persisted = persisted_generation_task_from_checkpoint(checkpoint.clone())?;
        let previous_snapshot = persisted.public_snapshot.ok_or_else(|| {
            GenerationTaskLifecycleError::Corrupt(format!(
                "active generation task {} 缺少 public snapshot",
                input.task_id
            ))
        })?;
        if previous_snapshot.started != input.public_snapshot.started {
            return Err(GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {} lifecycle started 发生变化",
                input.task_id
            )));
        }
        previous_snapshot.covers_through_sequence
    };
    if input.public_snapshot.covers_through_sequence <= previous_covers {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} lifecycle snapshot 未从当前 snapshot 向前推进",
            input.task_id
        )));
    }

    Ok(PreparedGenerationTransition {
        checkpoint,
        input,
        terminal,
    })
}

pub(super) async fn apply_prepared_transition<C>(
    db: &C,
    prepared: PreparedGenerationTransition,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    let PreparedGenerationTransition {
        checkpoint,
        input,
        terminal,
    } = prepared;

    let now = Utc::now().naive_utc();
    let covers = i64::try_from(input.public_snapshot.covers_through_sequence).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument(
            "lifecycle covers_through_sequence 超出 i64 范围".to_string(),
        )
    })?;
    validate_safe_sequence(covers, "covers_through_sequence")?;
    let reserved = extended_sequence_high_water(checkpoint.reserved_through_sequence, covers)?;
    let status = enum_storage(input.public_snapshot.status, "status")?;
    let public_snapshot_json = serde_json::to_value(&input.public_snapshot)
        .map_err(GenerationTaskPersistenceError::from)?;
    let public_snapshot_sha256 = canonical_json_sha256(&public_snapshot_json)
        .map_err(GenerationTaskPersistenceError::from)?;

    if terminal {
        insert_terminal_cleanup_intents(db, &checkpoint, &input.public_snapshot).await?;
    }

    let mut active: generation_task_checkpoint::ActiveModel = checkpoint.clone().into();
    active.status = Set(status.clone());
    active.checkpoint_revision = Set(checkpoint.checkpoint_revision + 1);
    active.covers_through_sequence = Set(covers);
    active.reserved_through_sequence = Set(reserved);
    active.public_snapshot_json = Set(Some(public_snapshot_json));
    active.public_snapshot_sha256 = Set(Some(public_snapshot_sha256));
    active.updated_at = Set(now);
    if terminal {
        active.resume_metadata_version = Set(None);
        active.execution_checkpoint_json = Set(None);
        active.execution_checkpoint_sha256 = Set(None);
        active.completed_at = Set(Some(now));
        active.expires_at = Set(Some(
            now + if input.public_snapshot.status == GenerationTaskStatus::Completed {
                COMPLETED_SNAPSHOT_TTL
            } else {
                RETAINED_DRAFT_TTL
            },
        ));
        let result = terminal_result_ref(&input.public_snapshot)?;
        active.result_kind = Set(result.as_ref().map(|result| result.0.clone()));
        active.result_id = Set(result.as_ref().map(|result| result.1.clone()));
        active.result_revision = Set(result.map(|result| result.2));
    }
    let checkpoint = active.update(db).await?;
    update_task_queue(
        db,
        &checkpoint.task_id,
        &status,
        terminal_message(&input.public_snapshot),
    )
    .await?;
    if terminal {
        update_workflow_run(
            db,
            &checkpoint,
            &status,
            terminal_message(&input.public_snapshot),
        )
        .await?;
        update_generation_run(
            db,
            &checkpoint,
            &status,
            terminal_message(&input.public_snapshot),
        )
        .await?;
    }
    Ok(PersistedGenerationCheckpoint {
        checkpoint_revision: checkpoint.checkpoint_revision,
        covers_through_sequence: checkpoint.covers_through_sequence,
        reserved_through_sequence: checkpoint.reserved_through_sequence,
        updated_at: checkpoint.updated_at,
    })
}

pub(super) fn extended_sequence_high_water(
    current: i64,
    required: i64,
) -> Result<i64, GenerationTaskLifecycleError> {
    if required <= current {
        return Ok(current);
    }
    let missing = required - current;
    let blocks = missing
        .checked_add(crate::generation::GENERATION_SEQUENCE_LEASE_SIZE - 1)
        .map(|value| value / crate::generation::GENERATION_SEQUENCE_LEASE_SIZE)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "generation sequence lease 扩展溢出".to_string(),
            )
        })?;
    current
        .checked_add(blocks * crate::generation::GENERATION_SEQUENCE_LEASE_SIZE)
        .filter(|value| *value <= JS_SAFE_INTEGER_I64)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "generation sequence lease 超过 JavaScript safe integer".to_string(),
            )
            .into()
        })
}

async fn update_task_queue<C>(
    db: &C,
    task_id: &str,
    status: &str,
    message: Option<String>,
) -> Result<(), GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let terminal = is_terminal_status(status);
    let queue_status = match status {
        "reserved" => "pending",
        "running" | "finalizing" | "cancelling" => "running",
        _ => status,
    };
    let mut update = task_queue::Entity::update_many()
        .col_expr(task_queue::Column::Status, Expr::value(queue_status))
        .col_expr(task_queue::Column::ErrorMessage, Expr::value(message))
        .filter(task_queue::Column::Id.eq(task_id.to_string()));
    if status == "running" {
        let started_at = CaseStatement::new()
            .case(
                task_queue::Column::StartedAt.is_null(),
                Expr::value(Some(now)),
            )
            .finally(Expr::col(task_queue::Column::StartedAt));
        update = update.col_expr(task_queue::Column::StartedAt, started_at.into());
    }
    if terminal {
        update = update.col_expr(task_queue::Column::CompletedAt, Expr::value(Some(now)));
    }
    let updated = update.exec(db).await?;
    if updated.rows_affected == 0 {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {task_id} 缺少 task_queue row"
        )));
    }
    if updated.rows_affected != 1 {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {task_id} 更新 task_queue 数量不一致: {}",
            updated.rows_affected
        )));
    }
    Ok(())
}

async fn update_workflow_run<C>(
    db: &C,
    checkpoint: &generation_task_checkpoint::Model,
    status: &str,
    message: Option<String>,
) -> Result<(), GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    let Some(workflow_run_id) = checkpoint.workflow_run_id else {
        return Ok(());
    };
    let Some((run_id, status_value, active_task_id)) =
        workflow_run::Entity::find_by_id(workflow_run_id)
            .select_only()
            .column(workflow_run::Column::Id)
            .column(workflow_run::Column::Status)
            .column(workflow_run::Column::ActiveTaskId)
            .lock_exclusive()
            .into_tuple::<(i64, String, Option<String>)>()
            .one(db)
            .await?
    else {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} 缺少 workflow_run {workflow_run_id}",
            checkpoint.task_id
        )));
    };
    if is_terminal_status(&status_value) {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    let clear_active_task = active_task_id.as_deref() == Some(checkpoint.task_id.as_str());
    let mut update = workflow_run::Entity::update_many()
        .col_expr(
            workflow_run::Column::Status,
            Expr::value(status.to_string()),
        )
        .col_expr(
            workflow_run::Column::CanResume,
            Expr::value(status == "interrupted"),
        )
        .col_expr(
            workflow_run::Column::ResumeBlockReason,
            Expr::value(Option::<String>::None),
        )
        .col_expr(workflow_run::Column::ErrorMessage, Expr::value(message))
        .col_expr(workflow_run::Column::UpdatedAt, Expr::value(now))
        .col_expr(workflow_run::Column::CompletedAt, Expr::value(Some(now)))
        .filter(workflow_run::Column::Id.eq(run_id));
    if clear_active_task {
        update = update.col_expr(
            workflow_run::Column::ActiveTaskId,
            Expr::value(Option::<String>::None),
        );
    }
    let updated = update.exec(db).await?;
    if updated.rows_affected != 1 {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} 更新 workflow_run {} 数量不一致: {}",
            checkpoint.task_id, run_id, updated.rows_affected
        )));
    }
    Ok(())
}

async fn update_generation_run<C>(
    db: &C,
    checkpoint: &generation_task_checkpoint::Model,
    status: &str,
    message: Option<String>,
) -> Result<(), GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    let Some(generation_run_id) = checkpoint.generation_run_id else {
        return Ok(());
    };
    let Some((run_id, status_value)) = generation_run::Entity::find_by_id(generation_run_id)
        .select_only()
        .column(generation_run::Column::Id)
        .column(generation_run::Column::Status)
        .lock_exclusive()
        .into_tuple::<(i64, String)>()
        .one(db)
        .await?
    else {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} 缺少 generation_run {generation_run_id}",
            checkpoint.task_id
        )));
    };
    if is_terminal_status(&status_value) {
        return Ok(());
    }
    let updated = generation_run::Entity::update_many()
        .col_expr(
            generation_run::Column::Status,
            Expr::value(status.to_string()),
        )
        .col_expr(generation_run::Column::ErrorMessage, Expr::value(message))
        .col_expr(
            generation_run::Column::CompletedAt,
            Expr::value(Some(Utc::now().naive_utc())),
        )
        .filter(generation_run::Column::Id.eq(run_id))
        .exec(db)
        .await?;
    if updated.rows_affected != 1 {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} 更新 generation_run {} 数量不一致: {}",
            checkpoint.task_id, run_id, updated.rows_affected
        )));
    }
    Ok(())
}
