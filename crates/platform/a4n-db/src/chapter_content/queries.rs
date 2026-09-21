use a4n_db_models::models::chapter::chapter;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use super::ChapterContentEditError;

pub async fn chapters_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<chapter::Model>, sea_orm::DbErr>
where
    C: ConnectionTrait,
{
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
}

/// 按 id 读取章节；不存在时返回 NotFound（带章节 id）。
pub async fn chapter_by_id_for_analysis<C>(
    db: &C,
    chapter_id: i64,
) -> Result<chapter::Model, ChapterContentEditError>
where
    C: ConnectionTrait,
{
    chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .map_err(ChapterContentEditError::database)?
        .ok_or_else(|| ChapterContentEditError::NotFound(format!("章节不存在: {chapter_id}")))
}
