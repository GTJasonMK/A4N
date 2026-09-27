use a4n_db_models::models::chapter::chapter;
use chrono::NaiveDateTime;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, sea_query::Expr};

pub(crate) async fn delete_project_chapter_content<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    now: NaiveDateTime,
) -> Result<u64, DbErr> {
    // Reset removes content; workflow history and frozen sources retain chapter identities.
    let retired = chapter::Entity::update_many()
        .col_expr(chapter::Column::OutlineId, Expr::value(Option::<i64>::None))
        .col_expr(chapter::Column::Title, Expr::value(Option::<String>::None))
        .col_expr(chapter::Column::Content, Expr::value(""))
        .col_expr(chapter::Column::WordCount, Expr::value(0))
        .col_expr(
            chapter::Column::Status,
            Expr::value(chapter::STATUS_DELETED),
        )
        .col_expr(
            chapter::Column::Summary,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            chapter::Column::StyleTags,
            Expr::value(Option::<serde_json::Value>::None),
        )
        .col_expr(
            chapter::Column::AnalysisData,
            Expr::value(Option::<serde_json::Value>::None),
        )
        .col_expr(chapter::Column::UpdatedAt, Expr::value(now))
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .exec(db)
        .await?;
    Ok(retired.rows_affected)
}
