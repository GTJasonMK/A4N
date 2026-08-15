use super::types::InsertConflictInput;
use crate::source_sync::{
    SourceSyncConflictRecordInput, SourceSyncDbError, insert_source_sync_conflict_record,
};
use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use sea_orm::ConnectionTrait;

pub(super) async fn insert_conflict<C>(
    db: &C,
    run: &source_sync_run::Model,
    input: InsertConflictInput<'_>,
) -> Result<source_sync_conflict::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let InsertConflictInput {
        conflict_scope,
        target_kind,
        target_id,
        source_change_kind,
        local_state_kind,
        severity,
        source_summary_json,
        local_summary_json,
        affected_refs_json,
    } = input;
    insert_source_sync_conflict_record(
        db,
        run,
        SourceSyncConflictRecordInput {
            conflict_scope: conflict_scope.to_string(),
            target_kind: target_kind.to_string(),
            target_id,
            source_change_kind,
            local_state_kind,
            severity,
            source_summary_json,
            local_summary_json,
            affected_refs_json,
        },
    )
    .await
}
