use a4n_db_models::models::{
    chapter::chapter,
    project::{project_source_snapshot, source_snapshot_chapter, standalone_comic_text_source},
};
use a4n_domain::{chapter::ResolvedChapterSource, project::ProjectSourceMode, source_text_hash};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use super::{
    ProjectSourceIdentity, contracts::ResolveChapterSourceError,
    snapshot::snapshot_source_from_chapter,
};

pub(super) async fn resolve_standalone_source<C>(
    db: &C,
    project: &ProjectSourceIdentity,
    source_chapter_id: i64,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError>
where
    C: ConnectionTrait,
{
    if ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| {
        ResolveChapterSourceError::InvalidArgument(format!("项目 {}: {error}", project.id))
    })? != ProjectSourceMode::Standalone
    {
        return Err(ResolveChapterSourceError::InvalidArgument(
            "文本素材来源只适用于独立漫画项目".to_string(),
        ));
    }
    let (source, chapter) = futures::try_join!(
        standalone_comic_text_source::Entity::find()
            .filter(standalone_comic_text_source::Column::ProjectId.eq(project.id))
            .filter(standalone_comic_text_source::Column::ChapterId.eq(source_chapter_id))
            .one(db),
        chapter::Entity::find_by_id(source_chapter_id).one(db),
    )?;
    let source = source.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!(
            "独立漫画项目 {} 未包含文本来源章节 {}",
            project.id, source_chapter_id
        ))
    })?;
    let chapter = chapter.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!("来源章节不存在: {source_chapter_id}"))
    })?;
    if chapter.project_id != project.id {
        return Err(ResolveChapterSourceError::InvalidArgument(format!(
            "来源章节 {source_chapter_id} 不属于独立漫画项目 {}",
            project.id
        )));
    }
    let (snapshot, frozen_chapter) = futures::try_join!(
        project_source_snapshot::Entity::find_by_id(source.source_snapshot_id).one(db),
        source_snapshot_chapter::Entity::find_by_id(source.source_snapshot_chapter_id).one(db),
    )?;
    let snapshot = snapshot.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!(
            "独立漫画来源快照不存在: {}",
            source.source_snapshot_id
        ))
    })?;
    let frozen_chapter = frozen_chapter.ok_or_else(|| {
        ResolveChapterSourceError::NotFound(format!(
            "独立漫画来源快照章节不存在: {}",
            source.source_snapshot_chapter_id
        ))
    })?;
    let content_hash = source_text_hash(&frozen_chapter.content);
    if snapshot.root_project_id != project.id
        || frozen_chapter.snapshot_id != snapshot.id
        || frozen_chapter.root_project_id != project.id
        || frozen_chapter.source_chapter_id != chapter.id
        || frozen_chapter.content != chapter.content
        || chapter.content.trim().is_empty()
        || source.content_hash != content_hash
        || frozen_chapter.content_hash != content_hash
    {
        return Err(ResolveChapterSourceError::InvalidState(format!(
            "独立漫画来源章节 {} 内容为空或快照哈希不匹配",
            chapter.id
        )));
    }
    Ok(snapshot_source_from_chapter(
        project.id,
        project.id,
        snapshot.id,
        frozen_chapter,
    ))
}
