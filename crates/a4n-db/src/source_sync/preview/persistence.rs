use super::types::InsertConflictInput;
use crate::source_sync::{
    SourceSyncConflictRecordInput, SourceSyncDbError, insert_source_sync_conflict_records,
};
use a4n_db_models::models::source_sync::{source_sync_conflict, source_sync_run};
use sea_orm::ConnectionTrait;

pub(super) async fn insert_conflicts<C>(
    db: &C,
    run: &source_sync_run::Model,
    inputs: Vec<InsertConflictInput<'_>>,
) -> Result<Vec<source_sync_conflict::Model>, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let inputs = inputs
        .into_iter()
        .map(
            |InsertConflictInput {
                 conflict_scope,
                 target_kind,
                 target_id,
                 source_change_kind,
                 local_state_kind,
                 severity,
                 source_summary_json,
                 local_summary_json,
                 affected_refs_json,
             }| SourceSyncConflictRecordInput {
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
        .collect();
    insert_source_sync_conflict_records(db, run, inputs).await
}
