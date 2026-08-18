use a4n_core::{
    models::{project::novel_project, project::source_snapshot_chapter},
    source_spans::source_text_hash,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::{
    super::derived_project_family_context,
    contracts::{ResolveChapterSourceError, ResolvedChapterSource, ResolvedChapterSourceKind},
};

pub(super) async fn resolve_snapshot_source<C>(
    db: &C,
    project: &novel_project::Model,
    source_chapter_id: i64,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError>
where
    C: ConnectionTrait,
{
    let context = derived_project_family_context(db, project.id)
        .await
        .map_err(map_family_error)?;
    let snapshot_chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(context.source_snapshot.id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
        .one(db)
        .await?
        .ok_or_else(|| {
            ResolveChapterSourceError::NotFound(format!(
                "媒体项目 {} 的源快照 {} 不包含章节 {}",
                project.id, context.source_snapshot.id, source_chapter_id
            ))
        })?;
    if snapshot_chapter.content.trim().is_empty() {
        return Err(ResolveChapterSourceError::InvalidState(format!(
            "源快照章节 {} 正文为空",
            snapshot_chapter.id
        )));
    }
    let actual_hash = source_text_hash(&snapshot_chapter.content);
    if actual_hash != snapshot_chapter.content_hash {
        return Err(ResolveChapterSourceError::InvalidState(format!(
            "源快照章节 {} 内容哈希不一致",
            snapshot_chapter.id
        )));
    }
    Ok(snapshot_source_from_chapter(
        project.id,
        context.root_project.id,
        context.source_snapshot.id,
        snapshot_chapter,
    ))
}

pub(super) fn snapshot_source_from_chapter(
    requested_project_id: i64,
    root_project_id: i64,
    snapshot_id: i64,
    chapter: source_snapshot_chapter::Model,
) -> ResolvedChapterSource {
    ResolvedChapterSource {
        source_kind: ResolvedChapterSourceKind::Snapshot,
        requested_project_id,
        root_project_id,
        source_chapter_id: chapter.source_chapter_id,
        source_snapshot_id: Some(snapshot_id),
        source_snapshot_chapter_id: Some(chapter.id),
        chapter_number: chapter.chapter_number,
        title: chapter.title,
        summary: chapter.summary,
        analysis_data_json: chapter.analysis_data_json,
        content: chapter.content,
        source_revision_key: format!(
            "snapshot:{snapshot_id}:chapter:{}:{}",
            chapter.id, chapter.content_hash
        ),
        content_sha256: chapter.content_hash,
    }
}

fn map_family_error(error: super::super::ProjectFamilyDbError) -> ResolveChapterSourceError {
    match error {
        super::super::ProjectFamilyDbError::InvalidArgument(message) => {
            ResolveChapterSourceError::InvalidArgument(message)
        }
        super::super::ProjectFamilyDbError::NotFound(message) => {
            ResolveChapterSourceError::NotFound(message)
        }
        super::super::ProjectFamilyDbError::Database(error) => {
            ResolveChapterSourceError::Database(error)
        }
    }
}
