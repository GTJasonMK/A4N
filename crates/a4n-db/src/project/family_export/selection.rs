use super::{
    super::PROJECT_KIND_TEXT,
    types::{ProjectFamilyExportError, ProjectFamilyExportSelection},
};
use a4n_core::models::project::novel_project;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

pub async fn project_family_export_selection<C>(
    db: &C,
    root_project_id: i64,
    include_derived_projects: bool,
) -> Result<ProjectFamilyExportSelection, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let root = novel_project::Entity::find_by_id(root_project_id)
        .one(db)
        .await
        .map_err(ProjectFamilyExportError::Database)?
        .ok_or_else(|| {
            ProjectFamilyExportError::NotFound(format!("根文本项目不存在: {root_project_id}"))
        })?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectFamilyExportError::InvalidArgument(
            "项目族导出的根项目必须是文本项目".to_string(),
        ));
    }
    let derived_projects = if include_derived_projects {
        novel_project::Entity::find()
            .filter(novel_project::Column::RootProjectId.eq(root.id))
            .order_by_asc(novel_project::Column::Id)
            .all(db)
            .await
            .map_err(ProjectFamilyExportError::Database)?
    } else {
        Vec::new()
    };
    Ok(ProjectFamilyExportSelection {
        root,
        derived_projects,
    })
}

pub async fn derived_project_export_selection<C>(
    db: &C,
    derived_project_id: i64,
) -> Result<ProjectFamilyExportSelection, ProjectFamilyExportError>
where
    C: ConnectionTrait,
{
    let derived = novel_project::Entity::find_by_id(derived_project_id)
        .one(db)
        .await
        .map_err(ProjectFamilyExportError::Database)?
        .ok_or_else(|| {
            ProjectFamilyExportError::NotFound(format!("派生项目不存在: {derived_project_id}"))
        })?;
    if derived.project_kind == PROJECT_KIND_TEXT {
        return Err(ProjectFamilyExportError::InvalidArgument(
            "派生项目导出不能传入文本项目".to_string(),
        ));
    }
    let root_project_id =
        derived
            .root_project_id
            .ok_or_else(|| ProjectFamilyExportError::InvalidState {
                current: format!("派生项目 {derived_project_id} 缺少 root_project_id"),
                operation: "导出派生项目".to_string(),
            })?;
    let root = novel_project::Entity::find_by_id(root_project_id)
        .one(db)
        .await
        .map_err(ProjectFamilyExportError::Database)?
        .ok_or_else(|| {
            ProjectFamilyExportError::NotFound(format!(
                "派生项目引用的根文本项目不存在: {root_project_id}"
            ))
        })?;
    if root.project_kind != PROJECT_KIND_TEXT {
        return Err(ProjectFamilyExportError::InvalidState {
            current: format!("根项目 {root_project_id} 不是文本项目"),
            operation: "导出派生项目".to_string(),
        });
    }
    Ok(ProjectFamilyExportSelection {
        root,
        derived_projects: vec![derived],
    })
}
