use super::super::types::DeletionMarkListStatus;
use a4n_core::models::character::protagonist_deletion_mark;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn deletion_marks_by_profile<C>(
    db: &C,
    profile_id: i64,
    category: Option<&str>,
    status: DeletionMarkListStatus,
    deletion_threshold: i32,
) -> Result<Vec<protagonist_deletion_mark::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id));

    if let Some(category) = category {
        query = query.filter(protagonist_deletion_mark::Column::Category.eq(category));
    }

    match status {
        DeletionMarkListStatus::All => {}
        DeletionMarkListStatus::Pending => {
            query = query
                .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
                .filter(protagonist_deletion_mark::Column::ConsecutiveCount.lt(deletion_threshold));
        }
        DeletionMarkListStatus::Ready => {
            query = query
                .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
                .filter(
                    protagonist_deletion_mark::Column::ConsecutiveCount.gte(deletion_threshold),
                );
        }
        DeletionMarkListStatus::Executed => {
            query = query.filter(protagonist_deletion_mark::Column::IsExecuted.eq(true));
        }
    }

    query
        .order_by_desc(protagonist_deletion_mark::Column::Id)
        .all(db)
        .await
}

pub async fn pending_deletion_mark_for_attribute<C>(
    db: &C,
    profile_id: i64,
    category: &str,
    key: &str,
) -> Result<Option<protagonist_deletion_mark::Model>, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_deletion_mark::Entity::find()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .filter(protagonist_deletion_mark::Column::Category.eq(category))
        .filter(protagonist_deletion_mark::Column::Key.eq(key))
        .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
        .one(db)
        .await
}
