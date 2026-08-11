use a4n_core::models::character::foreshadowing_index;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DbBackend, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
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
        // 正文删除只撤销由该章形成的“已回收”事实；正式伏笔本身由用户显式删除。
        if item.payoff_chapter != Some(chapter_number) {
            continue;
        }
        result.changed_ids.push(item.id);
        let updated_at = std::cmp::max(now, item.updated_at + chrono::Duration::microseconds(1));
        let mut active: foreshadowing_index::ActiveModel = item.into();
        active.payoff_chapter = Set(None);
        active.status = Set("pending".to_string());
        active.updated_at = Set(updated_at);
        active.update(db).await?;
    }
    Ok(result)
}
