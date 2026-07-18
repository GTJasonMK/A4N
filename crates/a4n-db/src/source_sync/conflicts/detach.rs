use crate::source_sync::{SOURCE_SYNC_DETACHED, SourceSyncDbError};
use a4n_core::models::project::novel_project;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, sea_query::Expr,
};

pub async fn detach_source_sync_project_record<C>(
    db: &C,
    derived_project_id: i64,
) -> Result<u64, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let result = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::SourceSyncStatus,
            Expr::value(SOURCE_SYNC_DETACHED),
        )
        .col_expr(
            novel_project::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(novel_project::Column::Id.eq(derived_project_id))
        .exec(db)
        .await?;
    Ok(result.rows_affected)
}

pub async fn detach_derived_project_record<C>(
    db: &C,
    project: novel_project::Model,
) -> Result<novel_project::Model, SourceSyncDbError>
where
    C: ConnectionTrait,
{
    let mut active: novel_project::ActiveModel = project.into();
    active.source_sync_status = Set(SOURCE_SYNC_DETACHED.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(SourceSyncDbError::Database)
}
