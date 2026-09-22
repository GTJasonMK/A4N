use super::super::super::PROJECT_KIND_TEXT;
use super::contracts::{
    ProjectFamilyAccessRow, ProjectFamilyContext, ProjectFamilyDbError,
    ProjectFamilySnapshotAccessRow,
};
use a4n_db_models::models::project::{
    novel_project, project_source_snapshot, source_snapshot_chapter,
};
use sea_orm::sea_query::{Alias, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, Linked, QueryFilter, QuerySelect};

pub async fn derived_project_family_access_ids<C>(
    db: &C,
    project_id: i64,
) -> Result<(i64, i64), ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let project = project_family_access_row(db, project_id).await?;
    derived_project_family_access_ids_from_row(db, project).await
}

pub async fn project_or_derived_family_access_ids<C>(
    db: &C,
    project_id: i64,
) -> Result<(i64, Option<i64>), ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let (project, root_project) = project_family_access_with_root_row(db, project_id).await?;
    if project.project_kind == PROJECT_KIND_TEXT {
        return Ok((project.id, None));
    }
    let root_project_id = project.root_project_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument(format!(
            "派生项目缺少 root_project_id: {}",
            project.id
        ))
    })?;
    let root_project = root_project
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("项目不存在: {root_project_id}")))?;
    validate_derived_project_family_access(db, project, root_project)
        .await
        .map(|root_project_id| (project_id, Some(root_project_id)))
}

async fn derived_project_family_access_ids_from_row<C>(
    db: &C,
    project: ProjectFamilyAccessRow,
) -> Result<(i64, i64), ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let root_project_id = project.root_project_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument(format!(
            "派生项目缺少 root_project_id: {}",
            project.id
        ))
    })?;
    let root_project = project_family_access_row(db, root_project_id).await?;
    let project_id = project.id;
    validate_derived_project_family_access(db, project, root_project)
        .await
        .map(|root_project_id| (project_id, root_project_id))
}

async fn validate_derived_project_family_access<C>(
    db: &C,
    project: ProjectFamilyAccessRow,
    root_project: ProjectFamilyAccessRow,
) -> Result<i64, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    if root_project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "根项目必须是文本项目".to_string(),
        ));
    }
    if project.project_kind == PROJECT_KIND_TEXT {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "该操作只支持派生项目".to_string(),
        ));
    }
    if project.root_project_id != Some(root_project.id) {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "派生项目 {} 不属于根文本项目 {}",
            project.id, root_project.id
        )));
    }
    let source_snapshot_id = project.source_snapshot_id.ok_or_else(|| {
        ProjectFamilyDbError::InvalidArgument(format!(
            "派生项目缺少 source_snapshot_id: {}",
            project.id
        ))
    })?;
    let source_snapshot = project_source_snapshot::Entity::find_by_id(source_snapshot_id)
        .select_only()
        .column(project_source_snapshot::Column::Id)
        .column(project_source_snapshot::Column::RootProjectId)
        .into_model::<ProjectFamilySnapshotAccessRow>()
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
    Ok(root_project.id)
}

struct ProjectFamilyRootLink;

impl Linked for ProjectFamilyRootLink {
    type FromEntity = novel_project::Entity;
    type ToEntity = novel_project::Entity;

    fn link(&self) -> Vec<sea_orm::LinkDef> {
        vec![
            novel_project::Entity::belongs_to(novel_project::Entity)
                .from(novel_project::Column::RootProjectId)
                .to(novel_project::Column::Id)
                .into(),
        ]
    }
}

async fn project_family_access_with_root_row<C>(
    db: &C,
    project_id: i64,
) -> Result<(ProjectFamilyAccessRow, Option<ProjectFamilyAccessRow>), ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find()
        .find_also_linked(ProjectFamilyRootLink)
        .select_only()
        .column_as(novel_project::Column::Id, "A_id")
        .column_as(novel_project::Column::ProjectKind, "A_project_kind")
        .column_as(novel_project::Column::RootProjectId, "A_root_project_id")
        .column_as(
            novel_project::Column::SourceSnapshotId,
            "A_source_snapshot_id",
        )
        .column_as(
            Expr::col((Alias::new("r0"), novel_project::Column::Id)),
            "B_id",
        )
        .column_as(
            Expr::col((Alias::new("r0"), novel_project::Column::ProjectKind)),
            "B_project_kind",
        )
        .column_as(
            Expr::col((Alias::new("r0"), novel_project::Column::RootProjectId)),
            "B_root_project_id",
        )
        .column_as(
            Expr::col((Alias::new("r0"), novel_project::Column::SourceSnapshotId)),
            "B_source_snapshot_id",
        )
        .filter(novel_project::Column::Id.eq(project_id))
        .into_model::<ProjectFamilyAccessRow, ProjectFamilyAccessRow>()
        .one(db)
        .await?
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("项目不存在: {project_id}")))
}

pub(super) async fn project_family_access_row<C>(
    db: &C,
    project_id: i64,
) -> Result<ProjectFamilyAccessRow, ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .into_model::<ProjectFamilyAccessRow>()
        .one(db)
        .await?
        .ok_or_else(|| ProjectFamilyDbError::NotFound(format!("项目不存在: {project_id}")))
}

pub(crate) async fn validate_project_family_source_context<C>(
    db: &C,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<(), ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let project = project_family_access_row(db, project_id).await?;
    let root_project = project_family_access_row(db, root_project_id).await?;
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
        .select_only()
        .column(project_source_snapshot::Column::Id)
        .column(project_source_snapshot::Column::RootProjectId)
        .into_model::<ProjectFamilySnapshotAccessRow>()
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
    Ok(())
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
    let (root_project_result, source_snapshot_result) = futures::join!(
        find_project_for_family(db, root_project_id),
        project_source_snapshot::Entity::find_by_id(source_snapshot_id).one(db),
    );
    let root_project = root_project_result?;
    if root_project.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectFamilyDbError::InvalidArgument(
            "根项目必须是文本项目".to_string(),
        ));
    }

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

    let source_snapshot = source_snapshot_result?.ok_or_else(|| {
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

pub(crate) async fn validate_project_family_snapshot_chapter<C>(
    db: &C,
    root_project_id: i64,
    source_snapshot_id: i64,
    source_snapshot_chapter_id: i64,
    source_chapter_id: Option<i64>,
) -> Result<(), ProjectFamilyDbError>
where
    C: ConnectionTrait,
{
    let chapter = source_snapshot_chapter::Entity::find_by_id(source_snapshot_chapter_id)
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SnapshotId)
        .column(source_snapshot_chapter::Column::RootProjectId)
        .column(source_snapshot_chapter::Column::SourceChapterId)
        .into_tuple::<(i64, i64, i64, i64)>()
        .one(db)
        .await?
        .ok_or_else(|| {
            ProjectFamilyDbError::NotFound(format!(
                "源文本快照章节不存在: {source_snapshot_chapter_id}"
            ))
        })?;
    if chapter.1 != source_snapshot_id {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "源文本快照章节 {} 不属于快照 {}",
            chapter.0, source_snapshot_id
        )));
    }
    if chapter.2 != root_project_id {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "源文本快照章节 {} 不属于根文本项目 {}",
            chapter.0, root_project_id
        )));
    }
    if let Some(source_chapter_id) = source_chapter_id
        && chapter.3 != source_chapter_id
    {
        return Err(ProjectFamilyDbError::InvalidArgument(format!(
            "源文本快照章节 {} 对应的源章节不是 {}",
            chapter.0, source_chapter_id
        )));
    }
    Ok(())
}

pub(super) async fn find_project_for_family<C>(
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
