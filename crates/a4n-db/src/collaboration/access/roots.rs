use a4n_core::models::project::novel_project;
use sea_orm::DatabaseConnection;

use super::super::{
    CollaborationDbError, CollaborationRootProject, find_project, validate_collaboration_root,
};

pub async fn resolve_root_project(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<CollaborationRootProject, CollaborationDbError> {
    let root = resolve_root_project_model(db, project_id).await?;
    Ok(CollaborationRootProject {
        id: root.id,
        user_id: root.user_id,
    })
}

pub(in crate::collaboration) async fn resolve_root_project_model(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<novel_project::Model, CollaborationDbError> {
    let project = find_project(db, project_id).await?;
    let root_id = project.root_project_id.unwrap_or(project.id);
    let root = if root_id == project.id {
        project
    } else {
        find_project(db, root_id).await?
    };
    validate_collaboration_root(&root)?;
    Ok(root)
}
