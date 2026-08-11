use super::validation::ensure_text_root_project;
use super::{
    CurrentChapterContentAnchorRows, SOURCE_KIND_CHAPTER_CONTENT, TextAnchorContentInsertInput,
    TextAnchorDbError, TextAnchorDbResult, insert_content_anchor_records,
};
use a4n_core::{
    models::{chapter::chapter, project::text_anchor},
    source_spans::source_text_hash,
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, TransactionTrait,
};

pub async fn rebuild_chapter_content_anchor_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> TextAnchorDbResult<Vec<text_anchor::Model>> {
    let project = ensure_text_root_project(db, project_id).await?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| TextAnchorDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
    if chapter.project_id != project.id {
        return Err(TextAnchorDbError::InvalidArgument(format!(
            "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
        )));
    }

    let tx = db.begin().await?;
    super::delete_chapter_content_profile_evidence_records(&tx, project.id, chapter.id).await?;
    text_anchor::Entity::delete_many()
        .filter(text_anchor::Column::ProjectId.eq(project.id))
        .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.eq(chapter.id))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .exec(&tx)
        .await?;

    let source_version_hash = source_text_hash(&chapter.content);
    let anchors = insert_content_anchor_records(
        &tx,
        TextAnchorContentInsertInput {
            project_id: project.id,
            source_chapter_id: Some(chapter.id),
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            source_id: chapter.id,
            chapter_number: chapter.chapter_number,
            content: &chapter.content,
            source_version_hash: &source_version_hash,
        },
    )
    .await?;
    tx.commit().await?;
    Ok(anchors)
}

pub async fn ensure_chapter_content_anchor_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> TextAnchorDbResult<Vec<text_anchor::Model>> {
    let project = ensure_text_root_project(db, project_id).await?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| TextAnchorDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
    if chapter.project_id != project.id {
        return Err(TextAnchorDbError::InvalidArgument(format!(
            "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
        )));
    }
    let source_version_hash = source_text_hash(&chapter.content);
    let existing = text_anchor::Entity::find()
        .filter(text_anchor::Column::ProjectId.eq(project.id))
        .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.eq(chapter.id))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .order_by_asc(text_anchor::Column::StartOffset)
        .all(db)
        .await?;
    if !existing.is_empty()
        && existing
            .iter()
            .all(|anchor| anchor.source_version_hash == source_version_hash)
    {
        return Ok(existing);
    }
    rebuild_chapter_content_anchor_records(db, project.id, chapter.id).await
}

pub async fn read_current_chapter_content_anchor_records(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> TextAnchorDbResult<CurrentChapterContentAnchorRows> {
    let project = ensure_text_root_project(db, project_id).await?;
    let chapter = chapter::Entity::find_by_id(chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| TextAnchorDbError::NotFound(format!("章节不存在: {chapter_id}")))?;
    if chapter.project_id != project.id {
        return Err(TextAnchorDbError::InvalidArgument(format!(
            "章节不属于文本根项目: chapter_id={chapter_id}, project_id={project_id}"
        )));
    }
    let source_version_hash = source_text_hash(&chapter.content);
    let anchors = text_anchor::Entity::find()
        .filter(text_anchor::Column::ProjectId.eq(project.id))
        .filter(text_anchor::Column::SourceKind.eq(SOURCE_KIND_CHAPTER_CONTENT))
        .filter(text_anchor::Column::SourceId.eq(chapter.id))
        .filter(text_anchor::Column::SourceSnapshotId.is_null())
        .order_by_asc(text_anchor::Column::StartOffset)
        .all(db)
        .await?;

    Ok(CurrentChapterContentAnchorRows {
        chapter,
        anchors,
        source_version_hash,
    })
}
