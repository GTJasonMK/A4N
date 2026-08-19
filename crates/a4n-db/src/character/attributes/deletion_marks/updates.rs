use super::super::types::CharacterDeletionMarkExecutedUpdateInput;
use a4n_core::models::character::protagonist_deletion_mark;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};

pub(in crate::character::attributes) async fn mark_deletion_executed<C>(
    db: &C,
    input: CharacterDeletionMarkExecutedUpdateInput,
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_deletion_mark::Entity::update_many()
        .col_expr(
            protagonist_deletion_mark::Column::IsExecuted,
            Expr::value(true),
        )
        .col_expr(
            protagonist_deletion_mark::Column::ConsecutiveCount,
            Expr::value(input.consecutive_count),
        )
        .col_expr(
            protagonist_deletion_mark::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(input.profile_id))
        .filter(protagonist_deletion_mark::Column::Category.eq(input.category))
        .filter(protagonist_deletion_mark::Column::Key.eq(input.key))
        .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
}

pub(in crate::character::attributes) async fn reset_pending_deletion_marks<C>(
    db: &C,
    profile_id: i64,
    category: &str,
    key: &str,
) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    protagonist_deletion_mark::Entity::delete_many()
        .filter(protagonist_deletion_mark::Column::ProfileId.eq(profile_id))
        .filter(protagonist_deletion_mark::Column::Category.eq(category))
        .filter(protagonist_deletion_mark::Column::Key.eq(key))
        .filter(protagonist_deletion_mark::Column::IsExecuted.eq(false))
        .exec(db)
        .await
        .map(|result| result.rows_affected)
}
