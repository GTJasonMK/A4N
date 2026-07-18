use a4n_core::models::chapter::chapter_outline;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
};

use super::contracts::ProjectOutlineRecordError;

pub async fn chapter_outline_by_project_number(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Option<chapter_outline::Model>, ProjectOutlineRecordError> {
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn chapter_outline_by_id<C>(
    db: &C,
    outline_id: i64,
) -> Result<Option<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find_by_id(outline_id)
        .one(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn chapter_outlines_for_project<C>(
    db: &C,
    project_id: i64,
) -> Result<Vec<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter_outline::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}

pub async fn chapter_outlines_for_project_chapter_number<C>(
    db: &C,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<chapter_outline::Model>, ProjectOutlineRecordError>
where
    C: ConnectionTrait,
{
    chapter_outline::Entity::find()
        .filter(chapter_outline::Column::ProjectId.eq(project_id))
        .filter(chapter_outline::Column::ChapterNumber.eq(chapter_number))
        .all(db)
        .await
        .map_err(ProjectOutlineRecordError::Database)
}
