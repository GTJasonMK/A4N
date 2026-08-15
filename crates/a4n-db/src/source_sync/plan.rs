use a4n_core::models::source_sync::{source_sync_conflict, source_sync_run};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::SourceSyncDbError;

#[derive(Debug, Clone)]
pub struct SourceSyncPlanRecord {
    pub run: source_sync_run::Model,
    pub conflicts: Vec<source_sync_conflict::Model>,
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
    Ok(SourceSyncPlanRecord { run, conflicts })
}
