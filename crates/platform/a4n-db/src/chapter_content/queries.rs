use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::ChapterRecord;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::ChapterContentEditError;

pub async fn chapters_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<ChapterRecord>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    chapter::Entity::find_current()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map(|chapters| {
            chapters
                .into_iter()
                .map(crate::project::chapter_record)
                .collect()
        })
}

/// 按 id 读取章节；不存在时返回 NotFound（带章节 id）。
pub async fn chapter_by_id_for_analysis<C>(
    db: &C,
    chapter_id: i64,
) -> Result<ChapterRecord, ChapterContentEditError>
where
    C: ConnectionTrait,
{
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
        .one(db)
        .await
        .map_err(ChapterContentEditError::database)?
        .ok_or_else(|| ChapterContentEditError::NotFound(format!("章节不存在: {chapter_id}")))?;
    Ok(crate::project::chapter_record(chapter))
}
