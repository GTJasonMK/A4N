use a4n_db_models::models::source_sync::{source_sync_conflict, source_sync_run};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use std::collections::HashMap;

use super::SourceSyncDbError;

#[derive(Debug, Clone)]
pub struct SourceSyncPlanRecord {
    pub run: source_sync_run::Model,
    pub conflicts: Vec<source_sync_conflict::Model>,
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
    Ok(SourceSyncPlanRecord {
        run,
        conflicts,
        regeneration_targets,
    })
}
