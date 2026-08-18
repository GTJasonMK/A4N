use super::lookups::find_user;
use crate::collaboration::{CollaborationDbError, ProjectAccessRole, ProjectMemberRole};
use a4n_core::models::{
    project::{novel_project, project_member},
    user::role::ROLE_READ_ONLY,
};
use a4n_domain::project::ProjectSourceMode;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub(in crate::collaboration) fn validate_collaboration_root(
    root: &novel_project::Model,
) -> Result<(), CollaborationDbError> {
    match root.source_mode().map_err(|error| {
        CollaborationDbError::InvalidArgument(format!("项目 {}: {error}", root.id))
    })? {
        ProjectSourceMode::Self_ | ProjectSourceMode::Freeform => Ok(()),
        ProjectSourceMode::Derived => Err(CollaborationDbError::InvalidArgument(
            "协作权限只能绑定到文本根项目或自由视频项目".to_string(),
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
    root: &novel_project::Model,
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
            .filter(project_member::Column::ProjectId.eq(root.id))
            .filter(project_member::Column::UserId.eq(user_id))
            .one(db)
            .await
            .map_err(CollaborationDbError::Database)?;
        match member {
            Some(member) => Some(parse_stored_member_role(&member.role)?.into()),
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
