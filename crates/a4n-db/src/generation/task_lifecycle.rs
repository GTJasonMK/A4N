use a4n_core::models::{
    generation::{
        generation_run, generation_start_request, generation_task_checkpoint, workflow_run,
    },
    task::{task_dependency, task_queue},
};
use a4n_service_protocol::{
    BatchCommitPolicy, BatchSummary, GenerationArtifactRef, GenerationEvent,
    GenerationTaskSnapshot, GenerationTaskStatus, JS_SAFE_INTEGER_I64, StartedEvent,
};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction,
    DbBackend, DbErr, EntityTrait, QueryFilter, QuerySelect, Set, Statement, TransactionTrait,
};
use sha2::Digest;

use crate::cleanup_outbox::{
    CLEANUP_AUDIT_TTL, GenerationArtifactUnbindPayload, GenerationCleanupScope,
    GenerationProviderJobCleanupOperation, GenerationProviderJobCleanupPayload,
    GenerationStagingArtifactDeletePayload, insert_generation_artifact_unbind_intent,
    insert_generation_provider_job_cleanup_intent,
    insert_generation_staging_artifact_delete_intent,
};

use super::canonical_json_sha256;
use super::lifecycle_gate::require_generation_owner_gates_active;
use super::tasks::{
    GenerationTaskPersistenceError, PersistedGenerationCheckpoint, canonical_uuid_v4,
    enum_from_storage, enum_storage, persisted_generation_task_from_checkpoint,
    started_event_from_checkpoint, validate_safe_sequence,
};

const COMPLETED_SNAPSHOT_TTL: Duration = Duration::hours(24);
const RETAINED_DRAFT_TTL: Duration = Duration::days(7);
const TERMINAL_STATUS_SQL: &str = "('completed', 'failed', 'cancelled', 'interrupted')";
const EXPIRED_GENERATION_ROOTS_SQL: &str = r#"
SELECT root.task_id
FROM generation_task_checkpoint AS root
WHERE root.root_task_id = root.task_id
  AND root.status IN ('completed', 'failed', 'cancelled', 'interrupted')
  AND root.expires_at IS NOT NULL
  AND root.expires_at <= NOW()
  AND EXISTS (
      SELECT 1
      FROM "user" AS owner
      WHERE owner.id = root.requested_by_user_id
        AND owner.lifecycle_gate = 'active'
  )
  AND NOT EXISTS (
      SELECT 1
      FROM novel_project AS project
      WHERE project.id IN (root.project_id, root.root_project_id)
        AND project.lifecycle_gate <> 'active'
  )
ORDER BY root.expires_at, root.task_id
FOR UPDATE SKIP LOCKED
LIMIT $1
"#;
const LOCK_GENERATION_TREE_SQL: &str = r#"
SELECT task_id
FROM generation_task_checkpoint
WHERE root_task_id = $1
ORDER BY task_id
FOR UPDATE
"#;
const LOAD_GENERATION_TREE_SQL: &str = r#"
WITH RECURSIVE task_tree(task_id, parent_task_id, status, expires_at, depth, path) AS (
    SELECT task_id,
           parent_task_id,
           status,
           expires_at,
           0,
           ARRAY[task_id]::text[]
    FROM generation_task_checkpoint
    WHERE task_id = $1
      AND root_task_id = task_id
    UNION ALL
    SELECT child.task_id,
           child.parent_task_id,
           child.status,
           child.expires_at,
           task_tree.depth + 1,
           task_tree.path || child.task_id
    FROM generation_task_checkpoint AS child
    JOIN task_tree ON child.parent_task_id = task_tree.task_id
    WHERE child.root_task_id = $1
      AND NOT child.task_id = ANY(task_tree.path)
)
SELECT task_id, parent_task_id, status, expires_at, depth
FROM task_tree
ORDER BY depth DESC, task_id ASC
"#;
const RECENT_GENERATION_AUDIT_SQL: &str = r#"
SELECT EXISTS (
    SELECT 1
    FROM project_cleanup_outbox AS cleanup
    WHERE cleanup.task_id IN (
        SELECT task_id
        FROM generation_task_checkpoint
        WHERE root_task_id = $1
    )
      AND (
          cleanup.completed_at IS NULL
          OR cleanup.completed_at > $2
      )
) AS has_recent_audit
"#;

#[derive(Debug, Clone)]
pub struct PersistGenerationLifecycleInput {
    pub task_id: String,
    pub attempt: u32,
    pub expected_checkpoint_revision: i64,
    pub expected_status: GenerationTaskStatus,
    pub public_snapshot: GenerationTaskSnapshot,
}

#[derive(Debug, Clone)]
pub struct GenerationCorruptRecoveryState {
    pub task_id: String,
    pub attempt: u32,
    pub expected_checkpoint_revision: i64,
    pub expected_status: GenerationTaskStatus,
    pub reserved_through_sequence: u64,
    pub started: StartedEvent,
    pub batch_summary: Option<BatchSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationCancelTarget {
    pub task_id: String,
    pub attempt: u32,
    pub requested_by_user_id: i64,
    pub status: GenerationTaskStatus,
    pub checkpoint_revision: i64,
    pub root_task_id: String,
}

#[derive(Debug, thiserror::Error)]
pub enum GenerationTaskLifecycleError {
    #[error(transparent)]
    Persistence(#[from] GenerationTaskPersistenceError),
    #[error("generation lifecycle CAS 冲突: status={current_status}, revision={current_revision}")]
    Conflict {
        current_status: String,
        current_revision: i64,
    },
    #[error("generation task 无权访问: {0}")]
    Forbidden(String),
    #[error("generation task attempt 不匹配: expected={expected}, actual={actual}")]
    AttemptMismatch { expected: u32, actual: u32 },
    #[error("generation lifecycle 数据损坏: {0}")]
    Corrupt(String),
    #[error(transparent)]
    Database(#[from] DbErr),
}

impl GenerationTaskLifecycleError {
    pub fn code(&self) -> &'static str {
        match self {
            Self::Persistence(error) => error.code(),
            Self::Conflict { current_status, .. }
                if matches!(current_status.as_str(), "finalizing" | "completed") =>
            {
                "generation_cancel_too_late"
            }
            Self::Conflict { current_status, .. }
                if matches!(
                    current_status.as_str(),
                    "failed" | "cancelled" | "interrupted"
                ) =>
            {
                "generation_terminal_already_set"
            }
            Self::Conflict { .. } | Self::Database(_) => "generation_persist_failed",
            Self::Forbidden(_) => "generation_task_forbidden",
            Self::AttemptMismatch { .. } => "generation_attempt_mismatch",
            Self::Corrupt(_) => "generation_checkpoint_corrupt",
        }
    }
}

pub async fn generation_cancel_targets(
    db: &DatabaseConnection,
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
    super::tasks::validate_safe_id(requested_by_user_id, "requested_by_user_id")?;
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    let mut prepared = Vec::with_capacity(inputs.len());
    for input in inputs {
        prepared.push(prepare_transition(&tx, input, false, false, false).await?);
    }
    let mut persisted = Vec::with_capacity(prepared.len());
    for transition in prepared {
        persisted.push(apply_prepared_transition(&tx, transition).await?);
    }
    tx.commit().await?;
    Ok(persisted)
}

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

fn valid_status_changed_transition(
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

/// 清理已过期 terminal snapshot 正文；保留规范 hash、result 引用和审计行。
///
/// 只处理 terminal 行并使用 `SKIP LOCKED`，因此不会阻塞 terminal/checkpoint 写事务。
pub async fn expire_generation_snapshot_bodies(
    db: &DatabaseConnection,
    limit: usize,
) -> Result<u64, GenerationTaskLifecycleError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation snapshot TTL 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    if limit == 0 {
        return Ok(0);
    }
    let limit = i64::try_from(limit).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument(
            "generation snapshot TTL limit 超出 i64 范围".to_string(),
        )
    })?;
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            format!(
                r#"
WITH expired AS (
    SELECT task_id
    FROM generation_task_checkpoint
    WHERE status IN {TERMINAL_STATUS_SQL}
      AND expires_at IS NOT NULL
      AND expires_at <= NOW()
      AND public_snapshot_json IS NOT NULL
    ORDER BY expires_at, task_id
    FOR UPDATE SKIP LOCKED
    LIMIT $1
)
UPDATE generation_task_checkpoint AS checkpoint
SET public_snapshot_json = NULL,
    checkpoint_revision = checkpoint_revision + 1
FROM expired
WHERE checkpoint.task_id = expired.task_id
RETURNING checkpoint.task_id
"#
            ),
            [limit.into()],
        ))
        .await?;
    Ok(rows.len() as u64)
}

/// 删除已经完成保留期且没有未完成 cleanup 审计的整棵 generation task tree。
///
/// root 行先由 `FOR UPDATE SKIP LOCKED` 领取；随后在同一事务中锁住整棵树，
/// 删除顺序固定为 start-request mapping → task dependency → 深度降序 task_queue。
/// checkpoint 依靠 task_queue 外键级联删除，避免留下 child checkpoint 或 mapping 孤儿。
pub async fn purge_expired_generation_task_trees(
    db: &DatabaseConnection,
    limit: usize,
) -> Result<u64, GenerationTaskLifecycleError> {
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(GenerationTaskPersistenceError::InvalidArgument(
            "generation task tree TTL 仅支持 PostgreSQL".to_string(),
        )
        .into());
    }
    if limit == 0 {
        return Ok(0);
    }
    let limit = i64::try_from(limit).map_err(|_| {
        GenerationTaskPersistenceError::InvalidArgument(
            "generation task tree TTL limit 超出 i64 范围".to_string(),
        )
    })?;
    let tx = db.begin().await?;
    let roots = tx
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            EXPIRED_GENERATION_ROOTS_SQL,
            [limit.into()],
        ))
        .await?;
    let mut deleted_tasks = 0u64;
    let audit_cutoff = Utc::now().naive_utc() - CLEANUP_AUDIT_TTL;

    for root in roots {
        let root_task_id: String = root.try_get("", "task_id")?;
        let locked_rows = tx
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                LOCK_GENERATION_TREE_SQL,
                [root_task_id.clone().into()],
            ))
            .await?;
        let tree_rows = tx
            .query_all(Statement::from_sql_and_values(
                DbBackend::Postgres,
                LOAD_GENERATION_TREE_SQL,
                [root_task_id.clone().into()],
            ))
            .await?;
        if locked_rows.is_empty() || locked_rows.len() != tree_rows.len() {
            return Err(GenerationTaskLifecycleError::Corrupt(format!(
                "generation task tree {root_task_id} 谱系不完整，拒绝清理"
            )));
        }

        let now = Utc::now().naive_utc();
        let mut task_ids = Vec::with_capacity(tree_rows.len());
        let mut all_expired_terminal = true;
        for row in &tree_rows {
            let task_id: String = row.try_get("", "task_id")?;
            let status: String = row.try_get("", "status")?;
            let expires_at: Option<chrono::NaiveDateTime> = row.try_get("", "expires_at")?;
            task_ids.push(task_id);
            if !is_terminal_status(&status) || expires_at.is_none_or(|value| value > now) {
                all_expired_terminal = false;
            }
        }
        if !all_expired_terminal {
            continue;
        }

        let recent_audit = tx
            .query_one(Statement::from_sql_and_values(
                DbBackend::Postgres,
                RECENT_GENERATION_AUDIT_SQL,
                [root_task_id.clone().into(), audit_cutoff.into()],
            ))
            .await?
            .ok_or_else(|| {
                GenerationTaskLifecycleError::Corrupt(format!(
                    "generation task tree {root_task_id} 审计查询没有返回结果"
                ))
            })?
            .try_get("", "has_recent_audit")?;
        if recent_audit {
            continue;
        }

        generation_start_request::Entity::delete_many()
            .filter(generation_start_request::Column::TaskId.is_in(task_ids.clone()))
            .exec(&tx)
            .await?;
        task_dependency::Entity::delete_many()
            .filter(task_dependency::Column::TaskId.is_in(task_ids.clone()))
            .exec(&tx)
            .await?;
        task_dependency::Entity::delete_many()
            .filter(task_dependency::Column::DependsOnTaskId.is_in(task_ids))
            .exec(&tx)
            .await?;

        // SQL 已按深度降序、同深度 task id 升序返回；不要改成 root-only DELETE。
        for row in tree_rows {
            let task_id: String = row.try_get("", "task_id")?;
            let result = task_queue::Entity::delete_by_id(task_id.clone())
                .exec(&tx)
                .await?;
            if result.rows_affected != 1 {
                return Err(GenerationTaskLifecycleError::Corrupt(format!(
                    "generation task tree {root_task_id} 删除 task_queue {task_id} 时行数不为 1"
                )));
            }
            deleted_tasks += 1;
        }
    }

    tx.commit().await?;
    Ok(deleted_tasks)
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

pub async fn generation_interruption_batch_summary(
    db: &DatabaseConnection,
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

async fn corrupt_recovery_batch_summary(
    db: &DatabaseConnection,
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
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "batch generation task {} checkpoint 损坏且没有 child task，无法重建精确 summary",
            checkpoint.task_id
        )));
    }
    Ok(Some(batch_summary_from_child_rows(policy, rows)?))
}

fn validate_transition_input<C>(
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

fn validate_terminal_transition(
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

async fn persist_transition(
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

struct PreparedGenerationTransition {
    checkpoint: generation_task_checkpoint::Model,
    input: PersistGenerationLifecycleInput,
    terminal: bool,
}

async fn prepare_transition<C>(
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

fn prepare_transition_from_checkpoint(
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

async fn apply_prepared_transition<C>(
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

fn extended_sequence_high_water(
    current: i64,
    required: i64,
) -> Result<i64, GenerationTaskLifecycleError> {
    if required <= current {
        return Ok(current);
    }
    let missing = required - current;
    let blocks = missing
        .checked_add(super::GENERATION_SEQUENCE_LEASE_SIZE - 1)
        .map(|value| value / super::GENERATION_SEQUENCE_LEASE_SIZE)
        .ok_or_else(|| {
            GenerationTaskPersistenceError::InvalidArgument(
                "generation sequence lease 扩展溢出".to_string(),
            )
        })?;
    current
        .checked_add(blocks * super::GENERATION_SEQUENCE_LEASE_SIZE)
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
    let task = task_queue::Entity::find_by_id(task_id.to_string())
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {task_id} 缺少 task_queue row"
            ))
        })?;
    let started_at_missing = task.started_at.is_none();
    let now = Utc::now().naive_utc();
    let mut active: task_queue::ActiveModel = task.into();
    let terminal = is_terminal_status(status);
    active.status = Set(match status {
        "reserved" => "pending".to_string(),
        "running" | "finalizing" | "cancelling" => "running".to_string(),
        _ => status.to_string(),
    });
    active.error_message = Set(message);
    if status == "running" && started_at_missing {
        active.started_at = Set(Some(now));
    }
    if terminal {
        active.completed_at = Set(Some(now));
    }
    active.update(db).await?;
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
    let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} 缺少 workflow_run {workflow_run_id}",
            checkpoint.task_id
        )));
    };
    if is_terminal_status(&run.status) {
        return Ok(());
    }
    let now = Utc::now().naive_utc();
    let clear_active_task = run.active_task_id.as_deref() == Some(checkpoint.task_id.as_str());
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    if clear_active_task {
        active.active_task_id = Set(None);
    }
    active.can_resume = Set(status == "interrupted");
    active.resume_block_reason = Set(None);
    active.error_message = Set(message);
    active.updated_at = Set(now);
    active.completed_at = Set(Some(now));
    active.update(db).await?;
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
    let Some(run) = generation_run::Entity::find_by_id(generation_run_id)
        .lock_exclusive()
        .one(db)
        .await?
    else {
        return Err(GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} 缺少 generation_run {generation_run_id}",
            checkpoint.task_id
        )));
    };
    if is_terminal_status(&run.status) {
        return Ok(());
    }
    let mut active: generation_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    active.error_message = Set(message);
    active.completed_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await?;
    Ok(())
}

async fn insert_terminal_cleanup_intents<C>(
    db: &C,
    checkpoint: &generation_task_checkpoint::Model,
    snapshot: &GenerationTaskSnapshot,
) -> Result<(), GenerationTaskLifecycleError>
where
    C: ConnectionTrait,
{
    let scope = GenerationCleanupScope {
        requested_by_user_id: checkpoint.requested_by_user_id,
        project_id: checkpoint.project_id,
        task_id: checkpoint.task_id.clone(),
        attempt: snapshot.attempt,
    };
    let mut intents = Vec::new();
    if snapshot.status != GenerationTaskStatus::Completed {
        if let (Some(provider_kind), Some(provider_job_id), Some(idempotency_hash)) = (
            checkpoint.provider_kind.clone(),
            checkpoint.provider_job_id.clone(),
            checkpoint.provider_idempotency_key_sha256.clone(),
        ) {
            for operation in [
                GenerationProviderJobCleanupOperation::Cancel,
                GenerationProviderJobCleanupOperation::Reconcile,
            ] {
                let operation_wire = match operation {
                    GenerationProviderJobCleanupOperation::Cancel => "cancel",
                    GenerationProviderJobCleanupOperation::Delete => "delete",
                    GenerationProviderJobCleanupOperation::Reconcile => "reconcile",
                };
                let resource_hash =
                    format!("{:x}", sha2::Sha256::digest(provider_job_id.as_bytes()));
                intents.push(PreparedCleanupIntent::Provider {
                    dedupe_key: format!(
                        "generation:{}:{}:provider:{operation_wire}:{resource_hash}",
                        checkpoint.task_id, snapshot.attempt
                    ),
                    payload: GenerationProviderJobCleanupPayload::new(
                        operation,
                        provider_kind.clone(),
                        provider_job_id.clone(),
                        idempotency_hash.clone(),
                    )?,
                });
            }
        } else if checkpoint.provider_job_id.is_some() {
            return Err(GenerationTaskLifecycleError::Corrupt(format!(
                "generation task {} provider job cleanup identity 不完整",
                checkpoint.task_id
            )));
        }
    }
    for artifact in snapshot
        .artifacts
        .iter()
        .filter(|artifact| artifact.temporary)
    {
        intents.push(PreparedCleanupIntent::Unbind {
            dedupe_key: format!(
                "generation:{}:{}:artifact-unbind:{}:{}",
                checkpoint.task_id, snapshot.attempt, artifact.artifact_id, artifact.revision
            ),
            payload: GenerationArtifactUnbindPayload::from_artifact(artifact)?,
        });
        intents.push(PreparedCleanupIntent::StagingDelete {
            dedupe_key: format!(
                "generation:{}:{}:staging-delete:{}:{}",
                checkpoint.task_id, snapshot.attempt, artifact.artifact_id, artifact.revision
            ),
            payload: GenerationStagingArtifactDeletePayload::from_artifact(artifact)?,
        });
    }
    for artifact in checkpoint_staging_artifacts(checkpoint)? {
        intents.push(PreparedCleanupIntent::StagingDelete {
            dedupe_key: format!(
                "generation:{}:{}:staging-delete:{}:{}",
                checkpoint.task_id, snapshot.attempt, artifact.artifact_id, artifact.revision
            ),
            payload: GenerationStagingArtifactDeletePayload::from_artifact(&artifact)?,
        });
    }
    intents.sort_by(|left, right| left.dedupe_key().cmp(right.dedupe_key()));
    intents.dedup_by(|left, right| left.dedupe_key() == right.dedupe_key());
    for intent in intents {
        match intent {
            PreparedCleanupIntent::Provider {
                dedupe_key,
                payload,
            } => {
                insert_generation_provider_job_cleanup_intent(db, dedupe_key, &scope, payload)
                    .await?;
            }
            PreparedCleanupIntent::Unbind {
                dedupe_key,
                payload,
            } => {
                insert_generation_artifact_unbind_intent(db, dedupe_key, &scope, payload).await?;
            }
            PreparedCleanupIntent::StagingDelete {
                dedupe_key,
                payload,
            } => {
                insert_generation_staging_artifact_delete_intent(db, dedupe_key, &scope, payload)
                    .await?;
            }
        }
    }
    Ok(())
}

fn checkpoint_staging_artifacts(
    checkpoint: &generation_task_checkpoint::Model,
) -> Result<Vec<GenerationArtifactRef>, GenerationTaskLifecycleError> {
    let Some(value) = checkpoint
        .execution_checkpoint_json
        .as_ref()
        .and_then(|value| value.get("staging_artifacts"))
    else {
        return Ok(Vec::new());
    };
    serde_json::from_value(value.clone()).map_err(|error| {
        GenerationTaskLifecycleError::Corrupt(format!(
            "generation task {} staging artifact checkpoint 无效: {error}",
            checkpoint.task_id
        ))
    })
}

enum PreparedCleanupIntent {
    Provider {
        dedupe_key: String,
        payload: GenerationProviderJobCleanupPayload,
    },
    Unbind {
        dedupe_key: String,
        payload: GenerationArtifactUnbindPayload,
    },
    StagingDelete {
        dedupe_key: String,
        payload: GenerationStagingArtifactDeletePayload,
    },
}

impl PreparedCleanupIntent {
    fn dedupe_key(&self) -> &str {
        match self {
            Self::Provider { dedupe_key, .. }
            | Self::Unbind { dedupe_key, .. }
            | Self::StagingDelete { dedupe_key, .. } => dedupe_key,
        }
    }
}

fn terminal_result_ref(
    snapshot: &GenerationTaskSnapshot,
) -> Result<Option<(String, String, String)>, GenerationTaskLifecycleError> {
    let Some(terminal) = snapshot.terminal_event.as_ref() else {
        return Err(GenerationTaskLifecycleError::Corrupt(
            "terminal snapshot 缺少 terminal_event".to_string(),
        ));
    };
    Ok(match &terminal.event {
        GenerationEvent::Completed(event) => event.result_ref.as_ref().map(|result| {
            (
                result.kind.clone(),
                result.id.clone(),
                result.revision.clone(),
            )
        }),
        GenerationEvent::Failed(_)
        | GenerationEvent::Cancelled(_)
        | GenerationEvent::Interrupted(_) => None,
        _ => {
            return Err(GenerationTaskLifecycleError::Corrupt(
                "terminal snapshot 的 terminal_event 不是 terminal variant".to_string(),
            ));
        }
    })
}

fn terminal_message(snapshot: &GenerationTaskSnapshot) -> Option<String> {
    snapshot
        .terminal_event
        .as_ref()
        .and_then(|terminal| match &terminal.event {
            GenerationEvent::Failed(event) => Some(event.error.message.clone()),
            GenerationEvent::Cancelled(event) => event.summary.clone(),
            GenerationEvent::Interrupted(event) => event.summary.clone(),
            GenerationEvent::Completed(_) => None,
            _ => None,
        })
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_transitions_and_ttls_are_fixed() {
        assert!(valid_status_changed_transition(
            GenerationTaskStatus::Reserved,
            GenerationTaskStatus::Running
        ));
        assert!(valid_status_changed_transition(
            GenerationTaskStatus::Running,
            GenerationTaskStatus::Finalizing
        ));
        assert!(!valid_status_changed_transition(
            GenerationTaskStatus::Running,
            GenerationTaskStatus::Completed
        ));
        assert!(
            validate_terminal_transition(
                GenerationTaskStatus::Finalizing,
                GenerationTaskStatus::Completed
            )
            .is_ok()
        );
        assert!(
            validate_terminal_transition(
                GenerationTaskStatus::Running,
                GenerationTaskStatus::Cancelled
            )
            .is_err()
        );
        assert_eq!(COMPLETED_SNAPSHOT_TTL, Duration::hours(24));
        assert_eq!(RETAINED_DRAFT_TTL, Duration::days(7));
        assert_eq!(extended_sequence_high_water(257, 258).unwrap(), 513);
    }
}
