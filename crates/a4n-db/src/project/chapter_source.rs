mod contracts;
mod live;
mod snapshot;
#[cfg(test)]
mod tests;

use a4n_core::models::project::{novel_project, source_snapshot_chapter};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
};
use std::collections::HashMap;

use self::{
    live::resolve_live_source,
    snapshot::{resolve_snapshot_source, snapshot_source_from_chapter},
};
use super::validate_positive_i64;

pub use contracts::{ResolveChapterSourceError, ResolvedChapterSource};

/// 在同一个已校验的派生项目快照内批量解析章节来源。
///
/// 调用方应先完成项目族身份校验；这里只保留逐章节的存在、正文和内容哈希校验，
/// 并按输入顺序返回，保证上层仍按原章节顺序报告第一个错误。
pub async fn resolve_snapshot_sources<C>(
    db: &C,
    requested_project_id: i64,
    root_project_id: i64,
    snapshot_id: i64,
    source_chapter_ids: &[i64],
) -> Result<Vec<ResolvedChapterSource>, ResolveChapterSourceError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(requested_project_id, "project_id")
        .map_err(ResolveChapterSourceError::InvalidArgument)?;
    validate_positive_i64(root_project_id, "root_project_id")
        .map_err(ResolveChapterSourceError::InvalidArgument)?;
    validate_positive_i64(snapshot_id, "source_snapshot_id")
        .map_err(ResolveChapterSourceError::InvalidArgument)?;
    for source_chapter_id in source_chapter_ids {
        validate_positive_i64(*source_chapter_id, "source_chapter_id")
            .map_err(ResolveChapterSourceError::InvalidArgument)?;
    }
    if source_chapter_ids.is_empty() {
        return Ok(Vec::new());
    }

    let rows = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .filter(source_snapshot_chapter::Column::SourceChapterId.is_in(source_chapter_ids.to_vec()))
        .all(db)
        .await?;
    let rows_by_source = rows.into_iter().fold(
        HashMap::<i64, source_snapshot_chapter::Model>::new(),
        |mut rows_by_source, row| {
            rows_by_source.entry(row.source_chapter_id).or_insert(row);
            rows_by_source
        },
    );

    source_chapter_ids
        .iter()
        .map(|source_chapter_id| {
            let chapter = rows_by_source.get(source_chapter_id).ok_or_else(|| {
                ResolveChapterSourceError::NotFound(format!(
                    "媒体项目 {} 的源快照 {} 不包含章节 {}",
                    requested_project_id, snapshot_id, source_chapter_id
                ))
            })?;
            if chapter.content.trim().is_empty() {
                return Err(ResolveChapterSourceError::InvalidState(format!(
                    "源快照章节 {} 正文为空",
                    chapter.id
                )));
            }
            let actual_hash = a4n_core::source_spans::source_text_hash(&chapter.content);
            if actual_hash != chapter.content_hash {
                return Err(ResolveChapterSourceError::InvalidState(format!(
                    "源快照章节 {} 内容哈希不一致",
                    chapter.id
                )));
            }
            Ok(snapshot_source_from_chapter(
                requested_project_id,
                root_project_id,
                snapshot_id,
                chapter.clone(),
            ))
        })
        .collect()
}

#[derive(Debug, FromQueryResult)]
pub(super) struct ProjectSourceIdentity {
    id: i64,
    project_kind: String,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
}

/// 解析文本项目或派生媒体项目所绑定的唯一章节来源。
///
/// 自由媒体项目没有章节来源，必须由调用方走自由创作合同，不能回退到同 ID 章节。
pub async fn resolve_chapter_source<C>(
    db: &C,
    project_id: i64,
    source_chapter_id: i64,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(project_id, "project_id")
        .map_err(ResolveChapterSourceError::InvalidArgument)?;
    validate_positive_i64(source_chapter_id, "source_chapter_id")
        .map_err(ResolveChapterSourceError::InvalidArgument)?;
    let project = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_model::<ProjectSourceIdentity>()
        .one(db)
        .await?
        .ok_or_else(|| ResolveChapterSourceError::NotFound(format!("项目不存在: {project_id}")))?;

    match ProjectSourceMode::from_identity(
        &project.project_kind,
        project.root_project_id,
        project.source_snapshot_id,
    )
    .map_err(|error| {
        ResolveChapterSourceError::InvalidArgument(format!("项目 {project_id}: {error}"))
    })? {
        ProjectSourceMode::Self_ => resolve_live_source(db, &project, source_chapter_id).await,
        ProjectSourceMode::Derived => {
            resolve_snapshot_source(db, &project, source_chapter_id).await
        }
        ProjectSourceMode::Freeform => Err(ResolveChapterSourceError::InvalidArgument(format!(
            "自由视频项目 {project_id} 没有章节来源"
        ))),
    }
}
