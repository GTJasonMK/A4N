use a4n_db_models::models::source_sync::source_sync_run;
use a4n_domain::source_sync::SourceSyncRunRecord;
use sea_orm::{ConnectionTrait, DbErr, EntityTrait};

pub async fn source_sync_run_by_id<C>(
    db: &C,
    sync_run_id: i64,
) -> Result<Option<source_sync_run::Model>, DbErr>
where
    C: ConnectionTrait,
{
    source_sync_run::Entity::find_by_id(sync_run_id)
        .one(db)
        .await
}

pub async fn source_sync_run_record_by_id<C>(
    db: &C,
    sync_run_id: i64,
) -> Result<Option<SourceSyncRunRecord>, DbErr>
where
    C: ConnectionTrait,
{
    source_sync_run_by_id(db, sync_run_id).await.map(|run| {
        run.map(|run| SourceSyncRunRecord {
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
        })
    })
}
