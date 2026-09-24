use a4n_db_models::models::project::{
    novel_project, project_source_snapshot, source_snapshot_chapter,
};
use a4n_domain::{chapter::ResolvedChapterSource, source_text_hash};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

use super::{ProjectSourceIdentity, contracts::ResolveChapterSourceError};
use crate::project::PROJECT_KIND_TEXT;

pub(super) async fn resolve_snapshot_source<C>(
    db: &C,
    project: &ProjectSourceIdentity,
    source_chapter_id: i64,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError>
where
    C: ConnectionTrait,
{
    let root_project_id = project.root_project_id.ok_or_else(|| {
        ResolveChapterSourceError::InvalidArgument(format!(
            "派生项目缺少 root_project_id: {}",
            project.id
        ))
    })?;
    let snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        ResolveChapterSourceError::InvalidArgument(format!(
            "派生项目缺少 source_snapshot_id: {}",
            project.id
        ))
    })?;
    let (root_project_result, snapshot_result, snapshot_chapter_result) = futures::join!(
        novel_project::Entity::find_by_id(root_project_id)
            .select_only()
            .column(novel_project::Column::Id)
            .column(novel_project::Column::ProjectKind)
            .into_tuple::<(i64, String)>()
            .one(db),
        project_source_snapshot::Entity::find_by_id(snapshot_id)
            .select_only()
            .column(project_source_snapshot::Column::Id)
            .column(project_source_snapshot::Column::RootProjectId)
            .into_tuple::<(i64, i64)>()
            .one(db),
        source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
            .filter(source_snapshot_chapter::Column::SourceChapterId.eq(source_chapter_id))
            .one(db),
    );
    let root_project = root_project_result?.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!("项目不存在: {root_project_id}"))
    })?;
    if root_project.1 != PROJECT_KIND_TEXT {
        return Err(ResolveChapterSourceError::InvalidArgument(
            "根项目必须是文本项目".to_string(),
        ));
    }
    let snapshot = snapshot_result?.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!("源文本快照不存在: {snapshot_id}"))
    })?;
    if snapshot.1 != root_project.0 {
        return Err(ResolveChapterSourceError::InvalidArgument(format!(
            "源文本快照 {} 不属于根文本项目 {}",
            snapshot.0, root_project.0
        )));
    }
    let snapshot_chapter = snapshot_chapter_result?.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!(
            "媒体项目 {} 的源快照 {} 不包含章节 {}",
            project.id, snapshot.0, source_chapter_id
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
        root_project.0,
        snapshot.0,
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
