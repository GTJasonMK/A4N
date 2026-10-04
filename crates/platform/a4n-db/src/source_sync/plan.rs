use a4n_db_models::models::source_sync::{source_sync_conflict, source_sync_run};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

use super::SourceSyncDbError;
use a4n_domain::source_sync::{SourceSyncConflictRecord, SourceSyncRunRecord};

#[derive(Debug, Clone)]
pub struct SourceSyncPlanRecord {
    pub run: SourceSyncRunRecord,
    pub conflicts: Vec<SourceSyncConflictRecord>,
    pub regeneration_targets: HashMap<i64, i64>,
}

pub(super) async fn source_sync_plan_record<C>(
    db: &C,
    run: source_sync_run::Model,
) -> Result<SourceSyncPlanRecord, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let conflicts = source_sync_conflict::Entity::find()
        .filter(source_sync_conflict::Column::SyncRunId.eq(run.id))
        .order_by_asc(source_sync_conflict::Column::Id)
        .all(db)
        .await?;
    let regeneration_targets =
        super::galgame::galgame_scene_regeneration_targets(db, run.derived_project_id, &conflicts)
            .await?;
    let run_record = SourceSyncRunRecord {
        id: run.id,
        derived_project_id: run.derived_project_id,
        root_project_id: run.root_project_id,
        from_snapshot_id: run.from_snapshot_id,
        to_snapshot_id: run.to_snapshot_id,
        mode: run.mode,
        status: run.status,
        summary_json: run.summary_json,
        error_message: run.error_message,
        created_at: run.created_at,
        updated_at: run.updated_at,
    };
    let conflict_records = conflicts
        .into_iter()
        .map(|conflict| SourceSyncConflictRecord {
            id: conflict.id,
            sync_run_id: conflict.sync_run_id,
            derived_project_id: conflict.derived_project_id,
            root_project_id: conflict.root_project_id,
            from_snapshot_id: conflict.from_snapshot_id,
            to_snapshot_id: conflict.to_snapshot_id,
            conflict_scope: conflict.conflict_scope,
            target_kind: conflict.target_kind,
            target_id: conflict.target_id,
            source_change_kind: conflict.source_change_kind,
            local_state_kind: conflict.local_state_kind,
            severity: conflict.severity,
            status: conflict.status,
            resolution_strategy: conflict.resolution_strategy,
            source_summary_json: conflict.source_summary_json,
            local_summary_json: conflict.local_summary_json,
            affected_refs_json: conflict.affected_refs_json,
            decision_json: conflict.decision_json,
            created_at: conflict.created_at,
            resolved_at: conflict.resolved_at,
        })
        .collect();
    Ok(SourceSyncPlanRecord {
        run: run_record,
        conflicts: conflict_records,
        regeneration_targets,
    })
}
