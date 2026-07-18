mod contracts;
mod live;
mod snapshot;
#[cfg(test)]
mod tests;

use a4n_core::models::project::novel_project;
use sea_orm::{ConnectionTrait, EntityTrait};

use self::{live::resolve_live_source, snapshot::resolve_snapshot_source};
use super::{
    PROJECT_KIND_COMIC, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    validate_positive_i64,
};

pub use contracts::{ResolveChapterSourceError, ResolvedChapterSource, ResolvedChapterSourceKind};

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
        .one(db)
        .await?
        .ok_or_else(|| ResolveChapterSourceError::NotFound(format!("项目不存在: {project_id}")))?;

    match project.project_kind.as_str() {
        PROJECT_KIND_TEXT => resolve_live_source(db, &project, source_chapter_id).await,
        PROJECT_KIND_COMIC | PROJECT_KIND_VIDEO | PROJECT_KIND_GALGAME => {
            resolve_snapshot_source(db, &project, source_chapter_id).await
        }
        other => Err(ResolveChapterSourceError::InvalidArgument(format!(
            "项目 {project_id} 的 project_kind 不支持章节导演计划: {other}"
        ))),
    }
}
