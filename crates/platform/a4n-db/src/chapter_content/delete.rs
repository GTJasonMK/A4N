use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QuerySelect, QueryTrait,
    sea_query::Expr,
};

pub(crate) async fn delete_project_chapter_content<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    now: NaiveDateTime,
) -> Result<u64, DbErr> {
    let retained = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(project_id))
        .into_query();
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
        .filter(chapter::Column::Id.in_subquery(retained.clone()))
        .exec(db)
        .await?;
    let deleted = chapter::Entity::delete_many()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Id.not_in_subquery(retained))
        .exec(db)
        .await?;
    Ok(retired.rows_affected + deleted.rows_affected)
}
