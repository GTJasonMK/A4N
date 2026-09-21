use super::contracts::ProjectChapterContextError;
use a4n_db_models::models::chapter::chapter;
use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder};

pub async fn chapter_by_id(
    db: &DatabaseConnection,
    chapter_id: i64,
) -> Result<Option<chapter::Model>, DbErr> {
    chapter::Entity::find_by_id(chapter_id).one(db).await
}

pub async fn import_analysis_chapter_candidates(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<chapter::Model>, ProjectChapterContextError> {
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::Status.ne("deleted"))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}

pub async fn character_sync_chapters(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<Vec<chapter::Model>, ProjectChapterContextError> {
    Ok(chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)?
        .into_iter()
        .filter(|chapter| chapter.has_available_content() && !chapter.is_generating())
        .collect())
}

pub async fn chapters_before_number(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<Vec<chapter::Model>, ProjectChapterContextError> {
    chapter::Entity::find()
        .filter(chapter::Column::ProjectId.eq(project_id))
        .filter(chapter::Column::ChapterNumber.lt(chapter_number))
        .order_by_asc(chapter::Column::ChapterNumber)
        .all(db)
        .await
        .map_err(ProjectChapterContextError::Database)
}
