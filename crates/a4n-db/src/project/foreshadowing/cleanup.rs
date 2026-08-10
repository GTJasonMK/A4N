use a4n_core::models::character::foreshadowing_index;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DbBackend, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait,
};

use super::contracts::{ForeshadowingCleanupResult, ForeshadowingDbError};

pub async fn cleanup_foreshadowing_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<ForeshadowingCleanupResult, ForeshadowingDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if crate::project::project_by_id_for_update(db, project_id)
        .await?
        .is_none()
    {
        return Err(ForeshadowingDbError::ProjectNotFound { project_id });
    }
    let mut query = foreshadowing_index::Entity::find()
        .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
        .filter(
            Condition::any()
                .add(foreshadowing_index::Column::SetupChapter.eq(chapter_number))
                .add(foreshadowing_index::Column::PayoffChapter.eq(Some(chapter_number))),
        )
        .order_by_asc(foreshadowing_index::Column::Id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let affected = query.all(db).await?;
    let now = Utc::now().naive_utc();
    let mut result = ForeshadowingCleanupResult::default();
    for item in affected {
        if item.setup_chapter == chapter_number {
            result.stale_ids.push(item.id);
            foreshadowing_index::Entity::delete_by_id(item.id)
                .exec(db)
                .await?;
            continue;
        }
        result.changed_ids.push(item.id);
        let mut active: foreshadowing_index::ActiveModel = item.into();
        active.payoff_chapter = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(now);
        active.update(db).await?;
    }
    Ok(result)
}
