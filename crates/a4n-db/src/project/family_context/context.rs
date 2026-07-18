use super::super::PROJECT_KIND_TEXT;
use super::contracts::{ProjectFamilyContext, ProjectFamilyDbError};
use a4n_core::models::project::{novel_project, project_source_snapshot, source_snapshot_chapter};
use sea_orm::{ConnectionTrait, EntityTrait};

pub async fn project_family_context<C>(
    db: &C,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<ProjectFamilyContext, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let project = find_project_for_family(db, project_id).await?;
    let root_project = find_project_for_family(db, root_project_id).await?;
    if root_project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "根项目必须是文本项目".to_string(),
        ));
    }

    if project.project_kind == PROJECT_KIND_TEXT {
        if project.id != root_project.id {
            return Err(ProjectFamilyDbError::InvalidArgument(
                "文本项目的 root_project_id 必须等于自身项目 ID".to_string(),
            ));
        }
    } else {
        if project.root_project_id != Some(root_project.id) {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "派生项目 {} 不属于根文本项目 {}",
                project.id, root_project.id
            )));
        }
        if project.source_snapshot_id != Some(source_snapshot_id) {
            return Err(ProjectFamilyDbError::InvalidArgument(format!(
                "派生项目 {} 未绑定源文本快照 {}",
                project.id, source_snapshot_id
            )));
        }
    }

    let source_snapshot = project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!("源文本快照不存在: {source_snapshot_id}"))
        })?;
    if source_snapshot.root_project_id != root_project.id {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
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

pub async fn derived_project_family_context<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectFamilyContext, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let project = find_project_for_family(db, project_id).await?;
    if project.project_kind == PROJECT_KIND_TEXT {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "该操作只支持派生项目".to_string(),
        ));
    }
    let root_project_id = project.root_project_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument("派生项目缺少 root_project_id".to_string())
    })?;
    let source_snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument("派生项目缺少 source_snapshot_id".to_string())
    })?;
    project_family_context(db, project_id, root_project_id, source_snapshot_id).await
}

pub async fn validate_project_family_snapshot_chapter<C>(
    db: &C,
    context: &ProjectFamilyContext,
    source_snapshot_chapter_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<source_snapshot_chapter::Model, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let chapter = source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "源文本快照章节不存在: {source_snapshot_chapter_id}"
            ))
        })?;
    if chapter.snapshot_id != context.source_snapshot.id {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "源文本快照章节 {} 不属于快照 {}",
            chapter.id, context.source_snapshot.id
        )));
    }
    if chapter.root_project_id != context.root_project.id {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "源文本快照章节 {} 不属于根文本项目 {}",
            chapter.id, context.root_project.id
        )));
    }
    if let Some(source_chapter_id) = source_chapter_id
        && chapter.source_chapter_id != source_chapter_id
    {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "源文本快照章节 {} 对应的源章节不是 {}",
            chapter.id, source_chapter_id
        )));
    }
    Ok(chapter)
}

pub(in crate::project::family_context) async fn find_project_for_family<C>(
    db: &C,
    project_id: i64,
) -> Result<novel_project::Model, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await?
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("项目不存在: {project_id}")))
}
