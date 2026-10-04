use super::lookups::find_user;
use crate::collaboration::{
    CollaborationDbError, CollaborationRootProject, ProjectAccessRole, ProjectMemberRole,
};
use a4n_db_models::models::project::project_member;
use a4n_domain::auth::ROLE_READ_ONLY;
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

pub(in crate::collaboration) fn validate_collaboration_root_identity(
    project_id: i64,
    project_kind: &str,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
) -> Result<(), CollaborationDbError> {
    match ProjectSourceMode::from_identity(project_kind, root_project_id, source_snapshot_id)
        .map_err(|error| {
            CollaborationDbError::InvalidArgument(format!("项目 {}: {error}", project_id))
        })? {
        ProjectSourceMode::Self_ | ProjectSourceMode::Freeform | ProjectSourceMode::Standalone => {
            Ok(())
        }
        ProjectSourceMode::Derived => Err(CollaborationDbError::InvalidArgument(
            "协作权限只能绑定到文本根项目、独立画布或独立漫画项目".to_string(),
        )),
    }
}

pub(in crate::collaboration) fn parse_stored_member_role(
    role: &str,
) -> Result<ProjectMemberRole, CollaborationDbError> {
    role.parse().map_err(|error: a4n_domain::DomainError| {
        CollaborationDbError::InvalidArgument(error.to_string())
    })
}

pub(in crate::collaboration) async fn effective_role_for_root(
    db: &DatabaseConnection,
    root: &CollaborationRootProject,
    user_id: i64,
) -> Result<Option<ProjectAccessRole>, CollaborationDbError> {
    let user = find_user(db, user_id).await?;
    if !user.is_active {
        return Err(CollaborationDbError::Forbidden("账户已被禁用".to_string()));
    }

    let mut role = if root.user_id == user_id {
        Some(ProjectAccessRole::Owner)
    } else {
        let member = project_member::Entity::find()
            .select_only()
            .column(project_member::Column::Role)
            .filter(project_member::Column::ProjectId.eq(root.id))
            .filter(project_member::Column::UserId.eq(user_id))
            .into_tuple::<String>()
            .one(db)
            .await
            .map_err(CollaborationDbError::Database)?;
        match member {
            Some(role) => Some(parse_stored_member_role(&role)?.into()),
            None => None,
        }
    };

    if user.role == ROLE_READ_ONLY && role.is_some() {
        role = Some(ProjectAccessRole::Viewer);
    }
    Ok(role)
}

pub(in crate::collaboration) fn validate_positive_i64(
    value: i64,
    field: &str,
) -> Result<(), CollaborationDbError> {
    if value <= 0 {
        Err(CollaborationDbError::InvalidArgument(format!(
            "{field} 必须大于 0"
        )))
    } else {
        Ok(())
    }
}
