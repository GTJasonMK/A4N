use super::records::{insert_source_sync_decision_record, resolve_source_sync_conflict_record};
use super::types::SourceSyncDecisionRecordInput;
use crate::source_sync::{CONFLICT_STATUS_PENDING, SourceSyncDbError};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use a4n_domain::source_sync::{
    SourceSyncConflictSeverity, SourceSyncConflictStatus, SourceSyncDecisionActor,
    SourceSyncResolutionStrategy,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
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
    let mut applied = 0u64;
    for conflict in conflicts {
        let decision_payload = json!({
            "system_action": "apply_unconflicted_item",
            "source_change_kind": conflict.source_change_kind,
            "local_state_kind": conflict.local_state_kind,
        });
        let conflict_id = conflict.id;
        resolve_source_sync_conflict_record(
            db,
            conflict,
            SourceSyncResolutionStrategy::KeepLocal,
            SourceSyncConflictStatus::Resolved,
            decision_payload.clone(),
        )
        .await?;
        insert_source_sync_decision_record(
            db,
            SourceSyncDecisionRecordInput {
                conflict_id,
                decided_by: SourceSyncDecisionActor::System,
                strategy: SourceSyncResolutionStrategy::KeepLocal,
                decision_payload_json: decision_payload,
            },
        )
        .await?;
        applied += 1;
    }
    Ok(applied)
}
