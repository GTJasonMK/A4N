use a4n_db_models::models::character::foreshadowing_index;
use a4n_domain::character::ForeshadowingStatus;
use chrono::Utc;
use sea_orm::{
    ColumnTrait, Condition, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
    sea_query::{CaseStatement, Expr},
};

use super::contracts::ForeshadowingDbError;

pub async fn cleanup_foreshadowing_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<i64>, ForeshadowingDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    if crate::project::project_write_identity_for_update(db, project_id)
        .await?
        .is_none()
    {
        return Err(ForeshadowingDbError::ProjectNotFound { project_id });
    }
    let mut query = foreshadowing_index::Entity::find()
        .select_only()
        .column(foreshadowing_index::Column::Id)
        .column(foreshadowing_index::Column::PayoffChapter)
        .column(foreshadowing_index::Column::UpdatedAt)
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
    let affected = query
        .into_tuple::<(i64, Option<i32>, chrono::NaiveDateTime)>()
        .all(db)
        .await?;
    let now = Utc::now().naive_utc();
    let mut changed_ids = Vec::new();
    let mut updated_at_by_id = CaseStatement::new();
    for (id, payoff_chapter, updated_at) in affected {
        // 正文删除只撤销由该章形成的“已回收”事实；正式伏笔本身由用户显式删除。
        if payoff_chapter != Some(chapter_number) {
            continue;
        }
        changed_ids.push(id);
        let updated_at = std::cmp::max(now, updated_at + chrono::Duration::microseconds(1));
        updated_at_by_id =
            updated_at_by_id.case(foreshadowing_index::Column::Id.eq(id), updated_at);
    }
    if !changed_ids.is_empty() {
        foreshadowing_index::Entity::update_many()
            .col_expr(
                foreshadowing_index::Column::PayoffChapter,
                Expr::value(Option::<i32>::None),
            )
            .col_expr(
                foreshadowing_index::Column::Status,
                Expr::value(ForeshadowingStatus::Pending.as_str()),
            )
            .col_expr(
                foreshadowing_index::Column::UpdatedAt,
                updated_at_by_id
                    .finally(Expr::col(foreshadowing_index::Column::UpdatedAt))
                    .into(),
            )
            .filter(foreshadowing_index::Column::Id.is_in(changed_ids.clone()))
            .exec(db)
            .await?;
    }
    Ok(changed_ids)
}
