use super::*;
pub async fn generation_cancel_targets(
    db: &impl crate::DbExecutor,
    requested_by_user_id: i64,
    task_id: &str,
    attempt: u32,
) -> Result<Vec<GenerationCancelTarget>, GenerationTaskLifecycleError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation cancel 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    super::validate_safe_id(requested_by_user_id, "requested_by_user_id")?;
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
WITH RECURSIVE task_tree(task_id, attempt, requested_by_user_id, root_task_id, parent_task_id, status, checkpoint_revision, path) AS (
    SELECT task_id,
           attempt,
           requested_by_user_id,
           root_task_id,
	           parent_task_id,
	           status,
	           checkpoint_revision,
	           ARRAY[task_id]::text[]
    FROM generation_task_checkpoint
    WHERE task_id = $1

    UNION ALL

    SELECT child.task_id,
           child.attempt,
           child.requested_by_user_id,
           child.root_task_id,
	           child.parent_task_id,
	           child.status,
	           child.checkpoint_revision,
	           parent.path || child.task_id
    FROM generation_task_checkpoint AS child
    JOIN task_tree AS parent ON child.parent_task_id = parent.task_id
    WHERE NOT child.task_id = ANY(parent.path)
)
SELECT task_id, attempt, requested_by_user_id, root_task_id, status, checkpoint_revision
FROM task_tree
ORDER BY task_id
"#,
            [task_id.clone().into()],
        ))
        .await?;
    if rows.is_empty() {
        return Err(GenerationTaskPersistenceError::NotFound(format!(
            "generation task 不存在: {task_id}"
        ))
        .into());
    }

    let mut targets = Vec::new();
    let mut requested = None;
    for row in rows {
        let row_task_id: String = row.try_get("", "task_id")?;
        let row_attempt: i32 = row.try_get("", "attempt")?;
        let row_user_id: i64 = row.try_get("", "requested_by_user_id")?;
        let root_task_id: String = row.try_get("", "root_task_id")?;
        let status_text: String = row.try_get("", "status")?;
        let checkpoint_revision: i64 = row.try_get("", "checkpoint_revision")?;
        let row_attempt = u32::try_from(row_attempt).map_err(|_| {
            GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {row_task_id} attempt 为负数"
            ))
        })?;
        let status = enum_from_storage(&status_text, "status")?;
        if row_task_id == task_id {
            requested = Some((
                row_attempt,
                row_user_id,
                status,
                checkpoint_revision,
                root_task_id.clone(),
            ));
        }
        targets.push(GenerationCancelTarget {
            task_id: row_task_id,
            attempt: row_attempt,
            requested_by_user_id: row_user_id,
            status,
            checkpoint_revision,
            root_task_id,
        });
    }

    let Some((
        actual_attempt,
        actual_user_id,
        requested_status,
        requested_revision,
        requested_root_task_id,
    )) = requested
    else {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation cancel task tree 缺少请求 task {task_id}"
        )));
    };
    if actual_user_id != requested_by_user_id {
        return Err(GenerationTaskLifecycleError::Forbidden(task_id));
    }
    if actual_attempt != attempt {
        return Err(GenerationTaskLifecycleError::AttemptMismatch {
            expected: attempt,
            actual: actual_attempt,
        });
    }
    match requested_status {
        GenerationTaskStatus::Finalizing | GenerationTaskStatus::Completed => {
            return Err(GenerationTaskLifecycleError::Conflict {
                current_status: enum_storage(requested_status, "status")?,
                current_revision: requested_revision,
            });
        }
        status if status.is_terminal() => {
            return Err(GenerationTaskLifecycleError::Conflict {
                current_status: enum_storage(status, "status")?,
                current_revision: requested_revision,
            });
        }
        _ => {}
    }
    if targets.iter().any(|target| {
        target.requested_by_user_id != actual_user_id
            || target.root_task_id != requested_root_task_id
    }) {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {task_id} descendant owner/root 不一致"
        )));
    }
    targets.retain(|target| {
        matches!(
            target.status,
            GenerationTaskStatus::Reserved
                | GenerationTaskStatus::Running
                | GenerationTaskStatus::Cancelling
        )
    });
    Ok(targets)
}

pub async fn persist_generation_cancel_requested(
    db: &impl crate::DbExecutor,
    input: PersistGenerationLifecycleInput,
) -> Result<PersistedGenerationCheckpoint, GenerationTaskLifecycleError> {
    validate_transition_input(db, &input, false)?;
    if !matches!(
        input.expected_status,
        GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
    ) || input.public_snapshot.status != GenerationTaskStatus::Cancelling
    {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "cancel_requested 只允许 reserved/running→cancelling".to_string(),
        )
        .into());
    }
    persist_transition(db, input, false, false, false).await
}

pub async fn persist_generation_cancel_requested_batch(
    db: &impl crate::DbExecutor,
    inputs: Vec<PersistGenerationLifecycleInput>,
) -> Result<Vec<PersistedGenerationCheckpoint>, GenerationTaskLifecycleError> {
    let mut previous_task_id: Option<&str> = None;
    for input in &inputs {
        validate_transition_input(db, input, false)?;
        if !matches!(
            input.expected_status,
            GenerationTaskStatus::Reserved | GenerationTaskStatus::Running
        ) || input.public_snapshot.status != GenerationTaskStatus::Cancelling
        {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "cancel_requested batch 只允许 reserved/running→cancelling".to_string(),
            )
            .into());
        }
        if previous_task_id.is_some_and(|previous| previous >= input.task_id.as_str()) {
            return Err(GenerationTaskPersistenceError::InvalidArgument(
                "cancel_requested batch task_id 必须严格递增且不重复".to_string(),
            )
            .into());
        }
        previous_task_id = Some(&input.task_id);
    }
    if inputs.is_empty() {
        return Ok(Vec::new());
    }

    let tx = db.begin().await?;
    let task_ids = inputs
        .iter()
        .map(|input| input.task_id.clone())
        .collect::<Vec<_>>();
    let checkpoints = generation_task_checkpoint::Entity::find()
        .filter(generation_task_checkpoint::Column::TaskId.is_in(task_ids.clone()))
        .order_by_asc(generation_task_checkpoint::Column::TaskId)
        .lock_exclusive()
        .all(&tx)
        .await?;
    let mut checkpoints_by_id = checkpoints
        .into_iter()
        .map(|checkpoint| (checkpoint.task_id.clone(), checkpoint))
        .collect::<HashMap<_, _>>();
    let mut prepared = Vec::with_capacity(inputs.len());
    for input in inputs {
        let task_id = input.task_id.clone();
        let checkpoint = checkpoints_by_id.remove(&task_id).ok_or_else(|| {
            GenerationTaskPersistenceError::NotFound(format!(
                "generation task checkpoint 不存在: {task_id}"
            ))
        })?;
        prepared.push(prepare_transition_from_checkpoint(
            checkpoint, input, false, false,
        )?);
    }

    // cancel_requested is non-terminal: all rows share the same queue update and have no
    // per-row cleanup side effects, so collapse the N checkpoint/queue writes into two updates.
    let queue_ids = task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .filter(task_queue::Column::Id.is_in(task_ids.clone()))
        .order_by_asc(task_queue::Column::Id)
        .lock_exclusive()
        .into_tuple::<String>()
        .all(&tx)
        .await?
        .into_iter()
        .collect::<HashSet<_>>();
    if queue_ids.len() != task_ids.len() {
        let missing = task_ids
            .iter()
            .find(|task_id| !queue_ids.contains(*task_id))
            .cloned()
            .unwrap_or_else(|| task_ids[0].clone());
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {missing} 缺少 task_queue row"
        )));
    }

    let mut checkpoint_revision_case = CaseStatement::new();
    let mut covers_case = CaseStatement::new();
    let mut reserved_case = CaseStatement::new();
    let mut public_snapshot_case = CaseStatement::new();
    let mut public_snapshot_sha256_case = CaseStatement::new();
    let mut updated_at_case = CaseStatement::new();
    let mut persisted = Vec::with_capacity(prepared.len());
    for transition in prepared {
        let PreparedGenerationTransition {
            checkpoint,
            input,
            terminal: _,
        } = transition;
        let now = Utc::now().naive_utc();
        let covers =
            i64::try_from(input.public_snapshot.covers_through_sequence).map_err(|_| {
                GenerationTaskPersistenceError::InvalidArgument(
                    "lifecycle covers_through_sequence 超出 i64 范围".to_string(),
                )
            })?;
        validate_safe_sequence(covers, "covers_through_sequence")?;
        let reserved = extended_sequence_high_water(checkpoint.reserved_through_sequence, covers)?;
        let public_snapshot_json = serde_json::to_value(&input.public_snapshot)
            .map_err(GenerationTaskPersistenceError::from)?;
        let public_snapshot_sha256 = canonical_json_sha256(&public_snapshot_json)
            .map_err(GenerationTaskPersistenceError::from)?;
        let task_id = checkpoint.task_id.clone();
        checkpoint_revision_case = checkpoint_revision_case.case(
            generation_task_checkpoint::Column::TaskId.eq(task_id.clone()),
            Expr::value(checkpoint.checkpoint_revision + 1),
        );
        covers_case = covers_case.case(
            generation_task_checkpoint::Column::TaskId.eq(task_id.clone()),
            Expr::value(covers),
        );
        reserved_case = reserved_case.case(
            generation_task_checkpoint::Column::TaskId.eq(task_id.clone()),
            Expr::value(reserved),
        );
        public_snapshot_case = public_snapshot_case.case(
            generation_task_checkpoint::Column::TaskId.eq(task_id.clone()),
            Expr::value(Some(public_snapshot_json)).cast_as("jsonb"),
        );
        public_snapshot_sha256_case = public_snapshot_sha256_case.case(
            generation_task_checkpoint::Column::TaskId.eq(task_id.clone()),
            Expr::value(Some(public_snapshot_sha256)),
        );
        updated_at_case = updated_at_case.case(
            generation_task_checkpoint::Column::TaskId.eq(task_id),
            Expr::value(now),
        );
        persisted.push(PersistedGenerationCheckpoint {
            checkpoint_revision: checkpoint.checkpoint_revision + 1,
            covers_through_sequence: covers,
            reserved_through_sequence: reserved,
            updated_at: now,
        });
    }

    let updated = generation_task_checkpoint::Entity::update_many()
        .col_expr(
            generation_task_checkpoint::Column::Status,
            Expr::value("cancelling"),
        )
        .col_expr(
            generation_task_checkpoint::Column::CheckpointRevision,
            checkpoint_revision_case
                .finally(Expr::col(
                    generation_task_checkpoint::Column::CheckpointRevision,
                ))
                .into(),
        )
        .col_expr(
            generation_task_checkpoint::Column::CoversThroughSequence,
            covers_case
                .finally(Expr::col(
                    generation_task_checkpoint::Column::CoversThroughSequence,
                ))
                .into(),
        )
        .col_expr(
            generation_task_checkpoint::Column::ReservedThroughSequence,
            reserved_case
                .finally(Expr::col(
                    generation_task_checkpoint::Column::ReservedThroughSequence,
                ))
                .into(),
        )
        .col_expr(
            generation_task_checkpoint::Column::PublicSnapshotJson,
            public_snapshot_case
                .finally(Expr::col(
                    generation_task_checkpoint::Column::PublicSnapshotJson,
                ))
                .into(),
        )
        .col_expr(
            generation_task_checkpoint::Column::PublicSnapshotSha256,
            public_snapshot_sha256_case
                .finally(Expr::col(
                    generation_task_checkpoint::Column::PublicSnapshotSha256,
                ))
                .into(),
        )
        .col_expr(
            generation_task_checkpoint::Column::UpdatedAt,
            updated_at_case
                .finally(Expr::col(generation_task_checkpoint::Column::UpdatedAt))
                .into(),
        )
        .filter(generation_task_checkpoint::Column::TaskId.is_in(task_ids.clone()))
        .exec(&tx)
        .await?;
    if updated.rows_affected != task_ids.len() as u64 {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation cancel batch 更新 checkpoint 数量不一致: {}",
            updated.rows_affected
        )));
    }

    let queue_updated = task_queue::Entity::update_many()
        .col_expr(task_queue::Column::Status, Expr::value("running"))
        .col_expr(
            task_queue::Column::ErrorMessage,
            Expr::value(Option::<String>::None),
        )
        .filter(task_queue::Column::Id.is_in(task_ids))
        .exec(&tx)
        .await?;
    if queue_updated.rows_affected != persisted.len() as u64 {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation cancel batch 更新 task_queue 数量不一致: {}",
            queue_updated.rows_affected
        )));
    }
    tx.commit().await?;
    Ok(persisted)
}

pub async fn generation_interruption_batch_summary(
    db: &impl crate::DbExecutor,
    task_id: &str,
) -> Result<Option<BatchSummary>, GenerationTaskLifecycleError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation batch summary 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    let task_id = canonical_uuid_v4(task_id, "task_id")?;
    let persisted = super::load_generation_task(db, &task_id).await?;
    let snapshot = persisted.public_snapshot.ok_or_else(|| {
        GenerationTaskLifecycleError::Corrupt(format!(
            "batch generation task {task_id} 缺少 public snapshot"
        ))
    })?;
    let Some(policy) = snapshot.started.batch_policy else {
        return Ok(None);
    };
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
SELECT item_key, status
FROM generation_task_checkpoint
WHERE root_task_id = $1
  AND task_id <> $1
ORDER BY item_key, task_id
"#,
            [task_id.into()],
        ))
        .await?;
    if rows.is_empty() {
        let progress = snapshot.progress.ok_or_else(|| {
            GenerationTaskLifecycleError::Corrupt(
                "batch generation task 既无 child task，也无 total progress".to_string(),
            )
        })?;
        let total = progress
            .total
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| {
                GenerationTaskLifecycleError::Corrupt(
                    "batch generation task progress.total 缺失或超出 u32".to_string(),
                )
            })?;
        let succeeded = progress
            .current
            .and_then(|value| u32::try_from(value).ok())
            .filter(|value| *value <= total)
            .ok_or_else(|| {
                GenerationTaskLifecycleError::Corrupt(
                    "batch generation task progress.current 缺失或大于 total".to_string(),
                )
            })?;
        return Ok(Some(BatchSummary {
            policy,
            total,
            succeeded,
            failed: 0,
            cancelled: 0,
            skipped: total - succeeded,
            failed_item_keys: Vec::new(),
            failed_item_keys_truncated: false,
        }));
    }

    Ok(Some(batch_summary_from_child_rows(policy, rows)?))
}

fn batch_summary_from_child_rows(
    policy: BatchCommitPolicy,
    rows: Vec<sea_orm::QueryResult>,
) -> Result<BatchSummary, GenerationTaskLifecycleError> {
    let total = u32::try_from(rows.len()).map_err(|_| {
        GenerationTaskLifecycleError::Corrupt(
            "batch generation child task 数量超出 u32".to_string(),
        )
    })?;
    let mut succeeded = 0u32;
    let mut failed = 0u32;
    let mut cancelled = 0u32;
    let mut skipped = 0u32;
    let mut failed_item_keys = Vec::new();
    for row in rows {
        let status: String = row.try_get("", "status")?;
        match status.as_str() {
            "completed" => succeeded += 1,
            "failed" => {
                failed += 1;
                let item_key: String = row.try_get("", "item_key")?;
                failed_item_keys.push(item_key);
            }
            "cancelled" => cancelled += 1,
            "reserved" | "running" | "finalizing" | "cancelling" | "interrupted" => {
                skipped += 1;
            }
            other => {
                return Err(GenerationTaskLifecycleError::Corrupt(format!(
                    "batch generation child status 无效: {other}"
                )));
            }
        }
    }
    failed_item_keys.sort();
    failed_item_keys.dedup();
    if failed_item_keys.len() != failed as usize {
        return Err(GenerationTaskLifecycleError::Corrupt(
            "batch generation failed child item_key 缺失或重复".to_string(),
        ));
    }
    let failed_item_keys_truncated = failed_item_keys.len() > 256;
    failed_item_keys.truncate(256);
    Ok(BatchSummary {
        policy,
        total,
        succeeded,
        failed,
        cancelled,
        skipped,
        failed_item_keys,
        failed_item_keys_truncated,
    })
}

pub(super) async fn corrupt_recovery_batch_summary(
    db: &impl crate::DbExecutor,
    checkpoint: &generation_task_checkpoint::Model,
) -> Result<Option<BatchSummary>, GenerationTaskLifecycleError> {
    let Some(policy) = checkpoint
        .batch_policy
        .as_deref()
        .map(|value| enum_from_storage(value, "batch_policy"))
        .transpose()?
    else {
        return Ok(None);
    };
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            r#"
SELECT item_key, status
FROM generation_task_checkpoint
WHERE root_task_id = $1
  AND task_id <> $1
ORDER BY item_key, task_id
"#,
            [checkpoint.task_id.clone().into()],
        ))
        .await?;
    if rows.is_empty() {
        // The checkpoint is already being quarantined as corrupt. Keeping the
        // summary absent is more accurate than manufacturing a child count.
        return Ok(None);
    }
    match batch_summary_from_child_rows(policy, rows) {
        Ok(summary) => Ok(Some(summary)),
        Err(GenerationTaskLifecycleError::Corrupt(_)) => Ok(None),
        Err(error) => Err(error),
    }
}
