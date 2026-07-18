use std::collections::HashMap;

use a4n_core::models::{project::project_member, user::role::ROLE_READ_ONLY};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::super::{
    CollaborationDbError, ProjectAccessRole, effective_role_for_root, find_user,
    member_role_to_project_access_role, projects_by_id, validate_collaboration_root,
    validate_positive_i64,
};
use super::roots::resolve_root_project_model;

pub async fn verify_project_permission(
    db: &DatabaseConnection,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<ProjectAccessRole, CollaborationDbError> {
    let root = resolve_root_project_model(db, project_id).await?;
    let Some(role) = effective_role_for_root(db, &root, user_id).await? else {
        return Err(CollaborationDbError::Forbidden(
            "无权访问该项目".to_string(),
        ));
    };
    if !role.satisfies(required) {
        return Err(CollaborationDbError::Forbidden(format!(
            "项目权限不足，需要 {} 权限",
            required.as_str()
        )));
    }
    Ok(role)
}

pub async fn verify_project_permissions(
    db: &DatabaseConnection,
    project_ids: &[i64],
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<(), CollaborationDbError> {
    if project_ids.is_empty() {
        return Ok(());
    }
    let mut normalized_project_ids = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        validate_positive_i64(*project_id, "project_id")?;
        normalized_project_ids.push(*project_id);
    }
    normalized_project_ids.sort_unstable();
    normalized_project_ids.dedup();

    let projects_by_id_map = projects_by_id(db, &normalized_project_ids).await?;
    for project_id in &normalized_project_ids {
        if !projects_by_id_map.contains_key(project_id) {
            return Err(CollaborationDbError::NotFound(format!(
                "项目不存在: {project_id}"
            )));
        }
    }

    let mut root_ids = Vec::with_capacity(normalized_project_ids.len());
    for project_id in &normalized_project_ids {
        let project = projects_by_id_map
            .get(project_id)
            .expect("project existence checked above");
        root_ids.push(project.root_project_id.unwrap_or(project.id));
    }
    root_ids.sort_unstable();
    root_ids.dedup();

    let roots_by_id = projects_by_id(db, &root_ids).await?;
    for root_id in &root_ids {
        let root = roots_by_id
            .get(root_id)
            .ok_or_else(|| CollaborationDbError::NotFound(format!("项目不存在: {root_id}")))?;
        validate_collaboration_root(root)?;
    }

    let user = find_user(db, user_id).await?;
    if !user.is_active {
        return Err(CollaborationDbError::Forbidden("账户已被禁用".to_string()));
    }

    let members = project_member::Entity::find()
        .filter(project_member::Column::ProjectId.is_in(root_ids.iter().copied()))
        .filter(project_member::Column::UserId.eq(user_id))
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    let mut member_roles = HashMap::with_capacity(members.len());
    for member in members {
        member_roles.insert(
            member.project_id,
            member_role_to_project_access_role(&member.role)?,
        );
    }

    for project_id in &normalized_project_ids {
        let project = projects_by_id_map
            .get(project_id)
            .expect("project existence checked above");
        let root_id = project.root_project_id.unwrap_or(project.id);
        let root = roots_by_id
            .get(&root_id)
            .expect("root existence checked above");
        let mut role = if root.user_id == user_id {
            Some(ProjectAccessRole::Owner)
        } else {
            member_roles.get(&root.id).copied()
        };
        if user.role == ROLE_READ_ONLY && role.is_some() {
            role = Some(ProjectAccessRole::Viewer);
        }
        let Some(role) = role else {
            return Err(CollaborationDbError::Forbidden(
                "无权访问该项目".to_string(),
            ));
        };
        if !role.satisfies(required) {
            return Err(CollaborationDbError::Forbidden(format!(
                "项目权限不足，需要 {} 权限",
                required.as_str()
            )));
        }
    }
    Ok(())
}
