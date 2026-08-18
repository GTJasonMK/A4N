use a4n_core::{
    models::{chapter::chapter, project::novel_project},
    source_spans::source_text_hash,
};
use sea_orm::{ConnectionTrait, EntityTrait};

use super::contracts::{
    ResolveChapterSourceError, ResolvedChapterSource, ResolvedChapterSourceKind,
};

pub(super) async fn resolve_live_source<C>(
    db: &C,
    project: &novel_project::Model,
    source_chapter_id: i64,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError>
where
    C: ConnectionTrait,
{
    let chapter = chapter::Entity::find_by_id(source_chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ResolveChapterSourceError::NotFound(format!(
                "源章节不存在: source_chapter_id={source_chapter_id}"
            ))
        })?;
    if chapter.project_id != project.id {
        return Err(ResolveChapterSourceError::InvalidArgument(format!(
            "源章节 {source_chapter_id} 不属于文本项目 {}",
            project.id
        )));
    }
    if chapter.status == chapter::STATUS_DELETED {
        return Err(ResolveChapterSourceError::InvalidState(format!(
            "源章节 {source_chapter_id} 已删除"
        )));
    }
    if chapter.status == chapter::STATUS_GENERATING {
        return Err(ResolveChapterSourceError::InvalidState(format!(
            "源章节 {source_chapter_id} 正在生成，不能建立导演计划"
        )));
    }
    if !chapter.has_content() {
        return Err(ResolveChapterSourceError::InvalidState(format!(
            "源章节 {source_chapter_id} 正文为空"
        )));
    }
    Ok(live_source_from_chapter(project.id, chapter))
}

pub(super) fn live_source_from_chapter(
    requested_project_id: i64,
    chapter: chapter::Model,
) -> ResolvedChapterSource {
    let content_sha256 = source_text_hash(&chapter.content);
    ResolvedChapterSource {
        source_kind: ResolvedChapterSourceKind::Live,
        requested_project_id,
        root_project_id: requested_project_id,
        source_chapter_id: chapter.id,
        source_snapshot_id: None,
        source_snapshot_chapter_id: None,
        chapter_number: chapter.chapter_number,
        title: chapter.title,
        summary: chapter.summary,
        analysis_data_json: chapter.analysis_data,
        content: chapter.content,
        source_revision_key: format!("live:{}:{content_sha256}", chapter.id),
        content_sha256,
    }
}
