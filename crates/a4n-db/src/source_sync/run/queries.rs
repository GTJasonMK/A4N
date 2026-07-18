use a4n_core::models::source_sync::source_sync_run;
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
