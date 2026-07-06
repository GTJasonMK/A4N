//! 项目族边界校验。

use sea_orm::*;

use a4n_core::error::AppError;
use a4n_core::models::project::{novel_project, project_source_snapshot, source_snapshot_chapter};

use crate::project::PROJECT_KIND_TEXT;

#[derive(Debug, Clone)]
pub struct ProjectFamilyContext {
    pub project: novel_project::Model,
    pub root_project: novel_project::Model,
    pub source_snapshot: project_source_snapshot::Model,
}

pub(crate) async fn load_project_family_context<C>(
    db: &C,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<ProjectFamilyContext, AppError>
where
    C: ConnectionTrait,
{
    let project = find_project(db, project_id).await?;
    let root_project = find_project(db, root_project_id).await?;
    if root_project.project_kind != PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(
            "根项目必须是文本项目".to_string(),
        ));
    }

    if project.project_kind == PROJECT_KIND_TEXT {
        if project.id != root_project.id {
            return Err(AppError::InvalidArgument(
                "文本项目的 root_project_id 必须等于自身项目 ID".to_string(),
            ));
        }
    } else {
        if project.root_project_id != Some(root_project.id) {
            return Err(AppError::InvalidArgument(format!(
                "派生项目 {} 不属于根文本项目 {}",
                project.id, root_project.id
            )));
        }
        if project.source_snapshot_id != Some(source_snapshot_id) {
            return Err(AppError::InvalidArgument(format!(
                "派生项目 {} 未绑定源文本快照 {}",
                project.id, source_snapshot_id
            )));
        }
    }

    let source_snapshot = project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("源文本快照不存在: {source_snapshot_id}")))?;
    if source_snapshot.root_project_id != root_project.id {
        return Err(AppError::InvalidArgument(format!(
            "源文本快照 {} 不属于根文本项目 {}",
            source_snapshot.id, root_project.id
        )));
    }

    Ok(ProjectFamilyContext {
        project,
        root_project,
        source_snapshot,
    })
}

pub async fn load_derived_project_context<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectFamilyContext, AppError>
where
    C: ConnectionTrait,
{
    let project = find_project(db, project_id).await?;
    if project.project_kind == PROJECT_KIND_TEXT {
        return Err(AppError::InvalidArgument(
            "该操作只支持派生项目".to_string(),
        ));
    }
    let root_project_id = project
        .root_project_id
        .ok_or_else(|| AppError::InvalidArgument("派生项目缺少 root_project_id".to_string()))?;
    let source_snapshot_id = project
        .source_snapshot_id
        .ok_or_else(|| AppError::InvalidArgument("派生项目缺少 source_snapshot_id".to_string()))?;
    load_project_family_context(db, project_id, root_project_id, source_snapshot_id).await
}

pub(crate) async fn validate_snapshot_chapter<C>(
    db: &C,
    context: &ProjectFamilyContext,
    source_snapshot_chapter_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<source_snapshot_chapter::Model, AppError>
where
    C: ConnectionTrait,
{
    let chapter = source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "源文本快照章节不存在: {source_snapshot_chapter_id}"
            ))
        })?;
    if chapter.snapshot_id != context.source_snapshot.id {
        return Err(AppError::InvalidArgument(format!(
            "源文本快照章节 {} 不属于快照 {}",
            chapter.id, context.source_snapshot.id
        )));
    }
    if chapter.root_project_id != context.root_project.id {
        return Err(AppError::InvalidArgument(format!(
            "源文本快照章节 {} 不属于根文本项目 {}",
            chapter.id, context.root_project.id
        )));
    }
    if let Some(source_chapter_id) = source_chapter_id
        && chapter.source_chapter_id != source_chapter_id
    {
        return Err(AppError::InvalidArgument(format!(
            "源文本快照章节 {} 对应的源章节不是 {}",
            chapter.id, source_chapter_id
        )));
    }
    Ok(chapter)
}

async fn find_project<C>(db: &C, project_id: i64) -> Result<novel_project::Model, AppError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
}
