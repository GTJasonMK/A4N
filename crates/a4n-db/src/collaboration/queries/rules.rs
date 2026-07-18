use super::lookups::find_user;
use crate::collaboration::types::{
    ACCESS_ROLE_OWNER, MEMBER_ROLE_EDITOR, MEMBER_ROLE_MANAGER, MEMBER_ROLE_VIEWER,
    PROJECT_KIND_COMIC, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    SOURCE_MODE_FREEFORM, SOURCE_MODE_SELF,
};
use crate::collaboration::{CollaborationDbError, ProjectAccessRole};
use a4n_core::models::{
    project::{novel_project, project_member},
    user::role::ROLE_READ_ONLY,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

pub(in crate::collaboration) fn validate_collaboration_root(
    root: &novel_project::Model,
) -> Result<(), CollaborationDbError> {
    if root.project_kind != PROJECT_KIND_TEXT
        && !(root.project_kind == PROJECT_KIND_VIDEO && root.source_mode == SOURCE_MODE_FREEFORM)
    {
        return Err(CollaborationDbError::InvalidArgument(
            "协作权限只能绑定到文本根项目或自由视频项目".to_string(),
        ));
    }
    if root.project_kind == PROJECT_KIND_TEXT && root.source_mode != SOURCE_MODE_SELF {
        return Err(CollaborationDbError::InvalidArgument(
            "文本协作根项目 source_mode 必须为 self".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::collaboration) fn normalize_project_kind(
    project_kind: &str,
) -> Result<&str, CollaborationDbError> {
    let project_kind = project_kind.trim();
    match project_kind {
        "" => Err(CollaborationDbError::InvalidArgument(
            "项目族类型不能为空".to_string(),
        )),
        PROJECT_KIND_TEXT | PROJECT_KIND_COMIC | PROJECT_KIND_VIDEO | PROJECT_KIND_GALGAME => {
            Ok(project_kind)
        }
        other => Err(CollaborationDbError::InvalidArgument(format!(
            "项目族类型无效: {other}"
        ))),
    }
}

pub(in crate::collaboration) fn member_role_to_access_role(
    role: &str,
) -> Result<String, CollaborationDbError> {
    Ok(member_role_to_project_access_role(role)?
        .as_str()
        .to_string())
}

pub(in crate::collaboration) fn member_role_to_project_access_role(
    role: &str,
) -> Result<ProjectAccessRole, CollaborationDbError> {
    match role.trim() {
        MEMBER_ROLE_MANAGER => Ok(ProjectAccessRole::Manager),
        MEMBER_ROLE_EDITOR => Ok(ProjectAccessRole::Editor),
        MEMBER_ROLE_VIEWER => Ok(ProjectAccessRole::Viewer),
        other => Err(CollaborationDbError::InvalidArgument(format!(
            "未知项目协作角色: {other}"
        ))),
    }
}

pub(in crate::collaboration) fn role_satisfies_manager(role: &str) -> bool {
    matches!(role, ACCESS_ROLE_OWNER | MEMBER_ROLE_MANAGER)
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
            Some(member) => Some(member_role_to_project_access_role(&member.role)?),
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
