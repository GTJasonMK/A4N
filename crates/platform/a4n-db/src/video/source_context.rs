use a4n_db_models::models::{chapter::chapter, project::source_snapshot_chapter};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::{
    VideoDbError, VideoProjectIdentity, validate_positive_video_i64, video_project_record,
};

pub async fn project_and_chapter_for_video_source<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<(VideoProjectIdentity, chapter::Model), VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    let (project_result, chapter_result) =
        futures::join!(video_project_record(db, project_id), async {
            let chapter = chapter::Entity::find_by_id(chapter_id)
                .one(db)
                .await
                .map_err(VideoDbError::Database)?;
            chapter.ok_or_else(|| {
                VideoDbError::NotFound(format!("章节不存在: chapter_id={chapter_id}"))
            })
        },);
    let project = project_result?;
    let chapter = chapter_result?;
    Ok((project, chapter))
}

pub async fn project_and_chapter_for_video_source_records<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<(VideoProjectIdentity, a4n_domain::chapter::ChapterRecord), VideoDbError> {
    project_and_chapter_for_video_source(db, project_id, chapter_id)
        .await
        .map(|(project, chapter)| (project, crate::project::chapter_record(chapter)))
}

pub async fn source_snapshot_chapter_for_video_source<C: ConnectionTrait>(
    db: &C,
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
