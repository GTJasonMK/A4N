use super::super::{
    CONFLICT_STATUS_PENDING, SourceSyncDbError, required_owned_text, validate_positive_id,
};
use super::types::{SourceSyncConflictRecordInput, SourceSyncDecisionRecordInput};
use a4n_db_models::models::source_sync::{
    source_sync_conflict, source_sync_decision, source_sync_run,
};
use a4n_domain::source_sync::{SourceSyncConflictStatus, SourceSyncResolutionStrategy};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, sea_query::Expr,
};
use serde_json::Value;
use std::collections::HashMap;

const SOURCE_SYNC_CONFLICT_INSERT_BATCH_SIZE: usize = 500;

pub async fn source_sync_conflict_for_decision<C>(
    db: &C,
    sync_run_id: i64,
    conflict_id: i64,
    operation: &str,
) -> Result<source_sync_conflict::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let conflict = source_sync_conflict::Entity::find_by_id(conflict_id)
        .one(db)
        .await?
        .ok_or_else(|| SourceSyncDbError::NotFound(format!("源同步冲突不存在: {conflict_id}")))?;
    if conflict.sync_run_id != sync_run_id {
        return Err(SourceSyncDbError::InvalidArgument(format!(
            "冲突 {} 不属于同步运行 {}",
            conflict.id, sync_run_id
        )));
    }
    if conflict.status != CONFLICT_STATUS_PENDING {
        return Err(SourceSyncDbError::InvalidState {
            current: conflict.status,
            operation: operation.to_string(),
        });
    }
    Ok(conflict)
}

pub async fn source_sync_conflicts_for_decisions<C>(
    db: &C,
    sync_run_id: i64,
    conflict_ids: &[i64],
    operation: &str,
) -> Result<HashMap<i64, source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    if conflict_ids.is_empty() {
        return Ok(HashMap::new());
    }

    let conflicts = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::Id.is_in(conflict_ids.iter().copied()))
        .all(db)
        .await?;
    let conflicts_by_id = conflicts
        .into_iter()
        .map(|conflict| (conflict.id, conflict))
        .collect::<HashMap<_, _>>();

    for conflict_id in conflict_ids {
        let conflict = conflicts_by_id.get(conflict_id).ok_or_else(|| {
            SourceSyncDbError::NotFound(format!("源同步冲突不存在: {conflict_id}"))
        })?;
        if conflict.sync_run_id != sync_run_id {
            return Err(SourceSyncDbError::InvalidArgument(format!(
                "冲突 {} 不属于同步运行 {}",
                conflict.id, sync_run_id
            )));
        }
        if conflict.status != CONFLICT_STATUS_PENDING {
            return Err(SourceSyncDbError::InvalidState {
                current: conflict.status.clone(),
                operation: operation.to_string(),
            });
        }
    }

    Ok(conflicts_by_id)
}

pub(in crate::source_sync) async fn insert_source_sync_conflict_records<C>(
    db: &C,
    run: &source_sync_run::Model,
    inputs: Vec<SourceSyncConflictRecordInput>,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    if inputs.is_empty() {
        return Ok(Vec::new());
    }
    validate_positive_id(run.id, "sync_run_id")?;
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;
    let now = Utc::now().naive_utc();
    let rows = inputs
        .into_iter()
        .map(|input| {
            validate_positive_id(input.target_id, "target_id")?;
            Ok(source_sync_conflict::ActiveModel {
                sync_run_id: Set(run.id),
                derived_project_id: Set(run.derived_project_id),
                root_project_id: Set(run.root_project_id),
                from_snapshot_id: Set(run.from_snapshot_id),
                to_snapshot_id: Set(run.to_snapshot_id),
                conflict_scope: Set(required_owned_text(input.conflict_scope, "conflict_scope")?),
                target_kind: Set(required_owned_text(input.target_kind, "target_kind")?),
                target_id: Set(input.target_id),
                source_change_kind: Set(input.source_change_kind.as_str().to_string()),
                local_state_kind: Set(input.local_state_kind.as_str().to_string()),
                severity: Set(input.severity.as_str().to_string()),
                status: Set(CONFLICT_STATUS_PENDING.to_string()),
                resolution_strategy: Set(None),
                source_summary_json: Set(input.source_summary_json),
                local_summary_json: Set(input.local_summary_json),
                affected_refs_json: Set(input.affected_refs_json),
                decision_json: Set(None),
                created_at: Set(now),
                resolved_at: Set(None),
                ..Default::default()
            })
        })
        .collect::<Result<Vec<_>, SourceSyncDbError>>()?;
    let expected = rows.len();
    let mut inserted = Vec::with_capacity(expected);
    for chunk in rows.chunks(SOURCE_SYNC_CONFLICT_INSERT_BATCH_SIZE) {
        inserted.extend(
            source_sync_conflict::Entity::insert_many(chunk.iter().cloned())
                .exec_with_returning_many(db)
                .await
                .map_err(SourceSyncDbError::Database)?,
        );
    }
    if inserted.len() != expected {
        return Err(SourceSyncDbError::Internal(format!(
            "批量写入源同步冲突返回 {} 行，预期 {} 行",
            inserted.len(),
            expected
        )));
    }
    Ok(inserted)
}

pub async fn resolve_source_sync_conflict_record<C>(
    db: &C,
    conflict: source_sync_conflict::Model,
    strategy: SourceSyncResolutionStrategy,
    status: SourceSyncConflictStatus,
    decision_json: Value,
) -> Result<source_sync_conflict::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    // Deleting the local target may already have retired this comparison in the same transaction.
    // Record the user's decision without turning that historical reference back into a live one.
    let mut updated = source_sync_conflict::Entity::update_many()
        .col_expr(
            source_sync_conflict::Column::Status,
            Expr::case(
                source_sync_conflict::Column::Status
                    .eq(SourceSyncConflictStatus::Obsolete.as_str()),
                SourceSyncConflictStatus::Obsolete.as_str(),
            )
            .finally(status.as_str())
            .into(),
        )
        .col_expr(
            source_sync_conflict::Column::ResolutionStrategy,
            Expr::value(Some(strategy.as_str().to_string())),
        )
        .col_expr(
            source_sync_conflict::Column::DecisionJson,
            Expr::value(Some(decision_json)),
        )
        .col_expr(
            source_sync_conflict::Column::ResolvedAt,
            Expr::value(Some(Utc::now().naive_utc())),
        )
        .filter(source_sync_conflict::Column::Id.eq(conflict.id))
        .exec_with_returning(db)
        .await?;
    updated
        .pop()
        .ok_or_else(|| SourceSyncDbError::NotFound(format!("源同步冲突不存在: {}", conflict.id)))
}

pub async fn insert_source_sync_decision_record<C>(
    db: &C,
    input: SourceSyncDecisionRecordInput,
) -> Result<source_sync_decision::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    source_sync_decision::ActiveModel {
        conflict_id: Set(input.conflict_id),
        decided_by: Set(input.decided_by.as_str().to_string()),
        strategy: Set(input.strategy.as_str().to_string()),
        decision_payload_json: Set(input.decision_payload_json),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(SourceSyncDbError::Database)
}
