mod contracts;
mod live;
mod snapshot;
mod standalone;
#[cfg(test)]
mod tests;

use a4n_db_models::models::project::{novel_project, source_snapshot_chapter};
use a4n_domain::{chapter::ResolvedChapterSource, project::ProjectSourceMode};
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect,
};
use std::collections::HashMap;

use self::{
    live::resolve_live_source,
    snapshot::{resolve_snapshot_source, snapshot_source_from_chapter},
    standalone::resolve_standalone_source,
};
use super::super::validate_positive_i64;

pub use contracts::ResolveChapterSourceError;

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
            let actual_hash = a4n_domain::source_text_hash(&chapter.content);
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

/// 在已验证的根项目和快照范围内一次解析全部章节来源。
///
/// 全量媒体编排需要正文、摘要和分析数据；直接读取所需字段并保留数据库顺序，
/// 避免先物化完整快照摘要再重复查询正文。
pub async fn resolve_snapshot_sources_for_scope<C>(
    db: &C,
    requested_project_id: i64,
    root_project_id: i64,
    snapshot_id: i64,
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
    let rows = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .column(source_snapshot_chapter::Column::ChapterNumber)
        .column(source_snapshot_chapter::Column::Title)
        .column(source_snapshot_chapter::Column::Content)
        .column(source_snapshot_chapter::Column::ContentHash)
        .filter(source_snapshot_chapter::Column::RootProjectId.eq(root_project_id))
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot_id))
        .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
        .into_tuple::<(i64, i64, i32, Option<String>, String, String)>()
        .all(db)
        .await?;
    rows.into_iter()
        .map(
            |(id, source_chapter_id, chapter_number, title, content, content_hash)| {
                if content.trim().is_empty() {
                    return Err(ResolveChapterSourceError::InvalidState(format!(
                        "源快照章节 {} 正文为空",
                        id
                    )));
                }
                let actual_hash = a4n_domain::source_text_hash(&content);
                if actual_hash != content_hash {
                    return Err(ResolveChapterSourceError::InvalidState(format!(
                        "源快照章节 {} 内容哈希不一致",
                        id
                    )));
                }
                Ok(ResolvedChapterSource {
                    requested_project_id,
                    root_project_id,
                    source_chapter_id,
                    source_snapshot_id: Some(snapshot_id),
                    source_snapshot_chapter_id: Some(id),
                    chapter_number,
                    title,
                    summary: None,
                    analysis_data_json: None,
                    content,
                    content_sha256: content_hash.clone(),
                    source_revision_key: format!(
                        "snapshot:{snapshot_id}:chapter:{id}:{content_hash}"
                    ),
                })
            },
        )
        .collect()
}

#[derive(Debug, FromQueryResult)]
pub(in crate::project) struct ProjectSourceIdentity {
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
    resolve_chapter_source_for_kind(db, project_id, source_chapter_id, None).await
}

pub async fn resolve_video_chapter_source<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    source_chapter_id: i64,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError> {
    resolve_chapter_source_for_kind(db, project_id, source_chapter_id, Some("video")).await
}

async fn resolve_chapter_source_for_kind<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    source_chapter_id: i64,
    required_kind: Option<&str>,
) -> Result<ResolvedChapterSource, ResolveChapterSourceError> {
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

    if required_kind.is_some_and(|kind| kind != project.project_kind) {
        return Err(ResolveChapterSourceError::InvalidArgument(
            "导演编排和拍摄剧本只能在视频派生项目中使用".into(),
        ));
    }

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
        ProjectSourceMode::Standalone => {
            resolve_standalone_source(db, &project, source_chapter_id).await
        }
        ProjectSourceMode::Freeform => Err(ResolveChapterSourceError::InvalidArgument(format!(
            "自由视频项目 {project_id} 没有章节来源"
        ))),
    }
}
