use crate::source_sync::{SOURCE_SYNC_DETACHED, SourceSyncDbError};
use a4n_db_models::models::project::novel_project;
use chrono::Utc;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, sea_query::Expr};

pub async fn detach_derived_project_record<C>(
    db: &C,
    derived_project_id: i64,
) -> Result<(), SourceSyncDbError>
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
    if result.rows_affected == 1 {
        Ok(())
    } else {
        Err(SourceSyncDbError::NotFound(format!(
            "派生项目不存在: {derived_project_id}"
        )))
    }
}
