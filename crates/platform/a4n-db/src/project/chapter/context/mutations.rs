use super::contracts::ProjectChapterContextError;
use a4n_db_models::models::{chapter::chapter, project::novel_project};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder, QuerySelect};

pub async fn refresh_project_current_chapter(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<(), ProjectChapterContextError> {
    let max_chapter = chapter::Entity::find()
        .select_only()
        .column(chapter::Column::ChapterNumber)
        .column(chapter::Column::Content)
        .column(chapter::Column::Status)
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_desc(chapter::Column::ChapterNumber)
        .into_tuple::<(i32, String, String)>()
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?
        .into_iter()
        .find(|(_, content, status)| !content.trim().is_empty() && status != "deleted")
        .map_or(0, |(chapter_number, _, _)| chapter_number);

    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::CurrentChapter)
        .into_tuple::<i32>()
        .one(db)
        .await
        .map_err(ProjectChapterContextError::Database)?
        .ok_or_else(|| {
            ProjectChapterContextError::NotFound(format!("项目不存在: project_id={project_id}"))
        })?;
    if project == max_chapter {
        return Ok(());
    }

    let updated = novel_project::Entity::update_many()
        .col_expr(
            novel_project::Column::CurrentChapter,
            Expr::value(max_chapter),
        )
        .col_expr(
            novel_project::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(novel_project::Column::Id.eq(project_id))
        .exec(db)
        .await
        .map_err(ProjectChapterContextError::Database)?;
    if updated.rows_affected != 1 {
        return Err(ProjectChapterContextError::NotFound(format!(
            "项目不存在: project_id={project_id}"
        )));
    }
    Ok(())
}
