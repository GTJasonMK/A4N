use a4n_core::models::{
    chapter::chapter,
    project::{novel_project, source_snapshot_chapter},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::{VideoDbError, validate_positive_video_i64};

pub async fn project_and_chapter_for_video_source(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<(novel_project::Model, chapter::Model), VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("项目不存在: {project_id}")))?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)?
        .ok_or_else(|| VideoDbError::NotFound(format!("章节不存在: chapter_id={chapter_id}")))?;
    Ok((project, chapter))
}

pub async fn source_snapshot_chapter_for_video_source(
    db: &DatabaseConnection,
    snapshot_id: i64,
    chapter_id: i64,
) -> Result<Option<source_snapshot_chapter::Model>, VideoDbError> {
    validate_positive_video_i64(snapshot_id, "snapshot_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(chapter_id))
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}
