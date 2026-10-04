use super::super::{
    CollaborationDbError, CollaborationRootProject, project_access_rows_by_id,
    validate_collaboration_root_identity,
};
use crate::collaboration::types::ProjectAccessRow;

pub async fn resolve_root_project(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<CollaborationRootProject, CollaborationDbError> {
    let project = project_access_row(db, project_id).await?;
    let root_id = project.root_project_id.unwrap_or(project.id);
    let root = if root_id == project.id {
        project
    } else {
        project_access_row(db, root_id).await?
    };
    validate_collaboration_root_identity(
        root.id,
        &root.project_kind,
        root.root_project_id,
        root.source_snapshot_id,
    )?;
    Ok(CollaborationRootProject {
        id: root.id,
        user_id: root.user_id,
    })
}

async fn project_access_row(
    db: &impl crate::DbExecutor,
    project_id: i64,
) -> Result<ProjectAccessRow, CollaborationDbError> {
    project_access_rows_by_id(db, &[project_id])
        .await?
        .remove(&project_id)
        .ok_or_else(|| CollaborationDbError::NotFound(format!("项目不存在: {project_id}")))
}
