use super::super::{
    CONFLICT_STATUS_PENDING, SourceSyncDbError, required_owned_text, required_text,
    validate_positive_id,
};
use super::types::{SourceSyncConflictRecordInput, SourceSyncDecisionRecordInput};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_decision, source_sync_run};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, EntityTrait, Set};
use serde_json::Value;

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

pub async fn insert_source_sync_conflict_record<C>(
    db: &C,
    run: &source_sync_run::Model,
    input: SourceSyncConflictRecordInput,
) -> Result<source_sync_conflict::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(run.id, "sync_run_id")?;
    validate_positive_id(run.derived_project_id, "derived_project_id")?;
    validate_positive_id(run.root_project_id, "root_project_id")?;
    validate_positive_id(run.from_snapshot_id, "from_snapshot_id")?;
    validate_positive_id(run.to_snapshot_id, "to_snapshot_id")?;
    validate_positive_id(input.target_id, "target_id")?;
    let conflict_scope = required_owned_text(input.conflict_scope, "conflict_scope")?;
    let target_kind = required_owned_text(input.target_kind, "target_kind")?;
    let source_change_kind = required_owned_text(input.source_change_kind, "source_change_kind")?;
    let local_state_kind = required_owned_text(input.local_state_kind, "local_state_kind")?;
    let severity = required_owned_text(input.severity, "severity")?;
    source_sync_conflict::ActiveModel {
        sync_run_id: Set(run.id),
        derived_project_id: Set(run.derived_project_id),
        root_project_id: Set(run.root_project_id),
        from_snapshot_id: Set(run.from_snapshot_id),
        to_snapshot_id: Set(run.to_snapshot_id),
        conflict_scope: Set(conflict_scope),
        target_kind: Set(target_kind),
        target_id: Set(input.target_id),
        source_change_kind: Set(source_change_kind),
        local_state_kind: Set(local_state_kind),
        severity: Set(severity),
        status: Set(CONFLICT_STATUS_PENDING.to_string()),
        resolution_strategy: Set(None),
        source_summary_json: Set(input.source_summary_json),
        local_summary_json: Set(input.local_summary_json),
        affected_refs_json: Set(input.affected_refs_json),
        decision_json: Set(None),
        created_at: Set(Utc::now().naive_utc()),
        resolved_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(SourceSyncDbError::Database)
}

pub async fn resolve_source_sync_conflict_record<C>(
    db: &C,
    conflict: source_sync_conflict::Model,
    strategy: &str,
    status: &str,
    decision_json: Value,
) -> Result<source_sync_conflict::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let mut active: source_sync_conflict::ActiveModel = conflict.into();
    active.status = Set(status.to_string());
    active.resolution_strategy = Set(Some(strategy.to_string()));
    active.decision_json = Set(Some(decision_json));
    active.resolved_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(SourceSyncDbError::Database)
}

pub async fn insert_source_sync_decision_record<C>(
    db: &C,
    input: SourceSyncDecisionRecordInput,
) -> Result<source_sync_decision::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let decided_by = required_text(&input.decided_by, "decided_by")?.to_string();
    let strategy = required_text(&input.strategy, "strategy")?.to_string();
    source_sync_decision::ActiveModel {
        conflict_id: Set(input.conflict_id),
        decided_by: Set(decided_by),
        strategy: Set(strategy),
        decision_payload_json: Set(input.decision_payload_json),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(SourceSyncDbError::Database)
}
