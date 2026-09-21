use super::*;
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
            .filter(task_dependency::Column::DependsOnTaskId.is_in(task_ids.clone()))
            .exec(&tx)
            .await?;

        // checkpoint 已锁定且依赖行已清理，按整棵树删除可保留级联语义并避免逐任务往返。
        let expected_tasks = task_ids.len() as u64;
        let deleted = task_queue::Entity::delete_many()
            .filter(task_queue::Column::Id.is_in(task_ids))
            .exec(&tx)
            .await?;
        if deleted.rows_affected != expected_tasks {
            return Err(GenerationTaskLifecycleError::Corrupt(format!(
                "generation task tree {root_task_id} 批量删除 task_queue 影响 {} 行，预期 {} 行",
                deleted.rows_affected, expected_tasks
            )));
        }
        deleted_tasks += deleted.rows_affected;
    }

    tx.commit().await?;
    Ok(deleted_tasks)
}
pub(super) async fn insert_terminal_cleanup_intents<C>(
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
                    )
                    .map_err(|error| {
                        GenerationTaskLifecycleError::Corrupt(format!(
                            "generation task {} provider cleanup payload 无效: {error}",
                            checkpoint.task_id
                        ))
                    })?,
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
            payload: GenerationArtifactUnbindPayload::from_artifact_parts(
                artifact.artifact_id.clone(),
                artifact.revision.clone(),
            )
            .map_err(|error| {
                GenerationTaskLifecycleError::Corrupt(format!(
                    "generation task {} artifact unbind payload 无效: {error}",
                    checkpoint.task_id
                ))
            })?,
        });
        intents.push(PreparedCleanupIntent::StagingDelete {
            dedupe_key: format!(
                "generation:{}:{}:staging-delete:{}:{}",
                checkpoint.task_id, snapshot.attempt, artifact.artifact_id, artifact.revision
            ),
            payload: GenerationStagingArtifactDeletePayload::from_artifact_parts(
                artifact.temporary,
                artifact.artifact_id.clone(),
                artifact.sha256.clone(),
                artifact.revision.clone(),
            )
            .map_err(|error| {
                GenerationTaskLifecycleError::Corrupt(format!(
                    "generation task {} staging artifact payload 无效: {error}",
                    checkpoint.task_id
                ))
            })?,
        });
    }
    for artifact in checkpoint_staging_artifacts(checkpoint)? {
        intents.push(PreparedCleanupIntent::StagingDelete {
            dedupe_key: format!(
                "generation:{}:{}:staging-delete:{}:{}",
                checkpoint.task_id, snapshot.attempt, artifact.artifact_id, artifact.revision
            ),
            payload: GenerationStagingArtifactDeletePayload::from_artifact_parts(
                artifact.temporary,
                artifact.artifact_id.clone(),
                artifact.sha256.clone(),
                artifact.revision.clone(),
            )
            .map_err(|error| {
                GenerationTaskLifecycleError::Corrupt(format!(
                    "generation task {} staging artifact payload 无效: {error}",
                    checkpoint.task_id
                ))
            })?,
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

pub(super) fn terminal_result_ref(
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

pub(super) fn terminal_message(snapshot: &GenerationTaskSnapshot) -> Option<String> {
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

pub(super) fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
}
