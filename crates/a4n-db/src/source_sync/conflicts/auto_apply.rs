use crate::source_sync::{CONFLICT_STATUS_PENDING, SourceSyncDbError};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_decision, source_sync_run};
use a4n_domain::source_sync::{
    SourceSyncConflictSeverity, SourceSyncConflictStatus, SourceSyncDecisionActor,
    SourceSyncResolutionStrategy,
};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set,
    sea_query::{Alias, CaseStatement, Expr},
};
use serde_json::json;

pub async fn apply_unconflicted_source_sync_records<C>(
    db: &C,
    run: &source_sync_run::Model,
) -> Result<u64, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let conflicts = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .filter(
            source_sync_conflict::Column::Severity.eq(SourceSyncConflictSeverity::Info.as_str()),
        )
        .all(db)
        .await?;
    if conflicts.is_empty() {
        return Ok(0);
    }

    let now = Utc::now().naive_utc();
    let mut conflict_ids = Vec::with_capacity(conflicts.len());
    let mut decision_json_case = CaseStatement::new();
    let mut decisions = Vec::with_capacity(conflicts.len());
    for conflict in &conflicts {
        let decision_payload = json!({
            "system_action": "apply_unconflicted_item",
            "source_change_kind": conflict.source_change_kind,
            "local_state_kind": conflict.local_state_kind,
        });
        conflict_ids.push(conflict.id);
        decision_json_case = decision_json_case.case(
            source_sync_conflict::Column::Id.eq(conflict.id),
            Expr::value(Some(decision_payload.clone())).cast_as(Alias::new("json")),
        );
        decisions.push(source_sync_decision::ActiveModel {
            conflict_id: Set(conflict.id),
            decided_by: Set(SourceSyncDecisionActor::System.as_str().to_string()),
            strategy: Set(SourceSyncResolutionStrategy::KeepLocal.as_str().to_string()),
            decision_payload_json: Set(decision_payload),
            created_at: Set(now),
            ..Default::default()
        });
    }

    let updated = source_sync_conflict::Entity::update_many()
        .col_expr(
            source_sync_conflict::Column::Status,
            Expr::value(SourceSyncConflictStatus::Resolved.as_str()),
        )
        .col_expr(
            source_sync_conflict::Column::ResolutionStrategy,
            Expr::value(Some(SourceSyncResolutionStrategy::KeepLocal.as_str())),
        )
        .col_expr(
            source_sync_conflict::Column::DecisionJson,
            decision_json_case
                .finally(Expr::col(source_sync_conflict::Column::DecisionJson))
                .into(),
        )
        .col_expr(
            source_sync_conflict::Column::ResolvedAt,
            Expr::value(Some(now)),
        )
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .filter(source_sync_conflict::Column::Id.is_in(conflict_ids.iter().copied()))
        .filter(source_sync_conflict::Column::Status.eq(CONFLICT_STATUS_PENDING))
        .filter(
            source_sync_conflict::Column::Severity.eq(SourceSyncConflictSeverity::Info.as_str()),
        )
        .exec(db)
        .await?;
    if updated.rows_affected != conflict_ids.len() as u64 {
        return Err(SourceSyncDbError::InvalidState {
            current: format!(
                "无阻塞源同步冲突批量更新影响 {} 行，预期 {} 行",
                updated.rows_affected,
                conflict_ids.len()
            ),
            operation: "自动应用无阻塞源同步冲突".to_string(),
        });
    }

    let inserted = source_sync_decision::Entity::insert_many(decisions)
        .exec_without_returning(db)
        .await?;
    if inserted != conflict_ids.len() as u64 {
        return Err(SourceSyncDbError::InvalidState {
            current: format!(
                "无阻塞源同步决策批量写入影响 {} 行，预期 {} 行",
                inserted,
                conflict_ids.len()
            ),
            operation: "记录自动源同步决策".to_string(),
        });
    }
    Ok(conflict_ids.len() as u64)
}
