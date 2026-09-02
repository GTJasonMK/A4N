use std::collections::HashMap;

use a4n_core::models::{project::project_member, user::role::ROLE_READ_ONLY};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};

use super::super::{
    CollaborationDbError, ProjectAccessRole, effective_role_for_root, find_user,
    parse_stored_member_role, project_access_rows_by_id, validate_collaboration_root_identity,
    validate_positive_i64,
};
use super::roots::resolve_root_project;

pub async fn verify_project_permission(
    db: &DatabaseConnection,
    project_id: i64,
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<ProjectAccessRole, CollaborationDbError> {
    let root = resolve_root_project(db, project_id).await?;
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

    let projects_by_id_map = project_access_rows_by_id(db, &normalized_project_ids).await?;
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

    let roots_by_id = project_access_rows_by_id(db, &root_ids).await?;
    for root_id in &root_ids {
        let root = roots_by_id
            .get(root_id)
            .ok_or_else(|| CollaborationDbError::NotFound(format!("项目不存在: {root_id}")))?;
        validate_collaboration_root_identity(
            root.id,
            &root.project_kind,
            root.root_project_id,
            root.source_snapshot_id,
        )?;
    }

    let (user_result, members_result) = futures::join!(find_user(db, user_id), async {
        project_member::Entity::find()
            .select_only()
            .column(project_member::Column::ProjectId)
            .column(project_member::Column::Role)
            .filter(project_member::Column::ProjectId.is_in(root_ids.iter().copied()))
            .filter(project_member::Column::UserId.eq(user_id))
            .into_tuple::<(i64, String)>()
            .all(db)
            .await
            .map_err(CollaborationDbError::Database)
    },);
    let user = user_result?;
    if !user.is_active {
        return Err(CollaborationDbError::Forbidden("账户已被禁用".to_string()));
    }

    let members = members_result?;
    let mut member_roles = HashMap::with_capacity(members.len());
    for (project_id, role) in members {
        member_roles.insert(project_id, parse_stored_member_role(&role)?.into());
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

/// 返回输入项目中满足权限要求的项目 ID。
///
/// 与 `verify_project_permissions` 不同，缺失或无权项目会被过滤，供列表查询保留
/// 原有的“逐条不可见”语义；数据库和项目身份错误仍然向上返回。
pub async fn project_ids_with_permission(
    db: &DatabaseConnection,
    project_ids: &[i64],
    user_id: i64,
    required: ProjectAccessRole,
) -> Result<Vec<i64>, CollaborationDbError> {
    if project_ids.is_empty() {
        return Ok(Vec::new());
    }

    let mut normalized_project_ids = Vec::with_capacity(project_ids.len());
    for project_id in project_ids {
        validate_positive_i64(*project_id, "project_id")?;
        normalized_project_ids.push(*project_id);
    }
    normalized_project_ids.sort_unstable();
    normalized_project_ids.dedup();

    let projects_by_id_map = project_access_rows_by_id(db, &normalized_project_ids).await?;
    let mut root_ids = normalized_project_ids
        .iter()
        .filter_map(|project_id| projects_by_id_map.get(project_id))
        .map(|project| project.root_project_id.unwrap_or(project.id))
        .collect::<Vec<_>>();
    root_ids.sort_unstable();
    root_ids.dedup();

    let mut roots_by_id = root_ids
        .iter()
        .filter_map(|root_id| {
            projects_by_id_map
                .get(root_id)
                .map(|root| (*root_id, root.clone()))
        })
        .collect::<HashMap<_, _>>();
    let missing_root_ids = root_ids
        .iter()
        .copied()
        .filter(|root_id| !roots_by_id.contains_key(root_id))
        .collect::<Vec<_>>();
    if !missing_root_ids.is_empty() {
        roots_by_id.extend(project_access_rows_by_id(db, &missing_root_ids).await?);
    }
    for root_id in &root_ids {
        if let Some(root) = roots_by_id.get(root_id) {
            validate_collaboration_root_identity(
                root.id,
                &root.project_kind,
                root.root_project_id,
                root.source_snapshot_id,
            )?;
        }
    }

    let (user_result, members_result) = futures::join!(find_user(db, user_id), async {
        if root_ids.is_empty() {
            Ok(Vec::new())
        } else {
            project_member::Entity::find()
                .select_only()
                .column(project_member::Column::ProjectId)
                .column(project_member::Column::Role)
                .filter(project_member::Column::ProjectId.is_in(root_ids.iter().copied()))
                .filter(project_member::Column::UserId.eq(user_id))
                .into_tuple::<(i64, String)>()
                .all(db)
                .await
                .map_err(CollaborationDbError::Database)
        }
    },);
    let user = user_result?;
    if !user.is_active {
        return Err(CollaborationDbError::Forbidden("账户已被禁用".to_string()));
    }

    let members = members_result?;
    let mut member_roles = HashMap::with_capacity(members.len());
    for (project_id, role) in members {
        member_roles.insert(project_id, parse_stored_member_role(&role)?.into());
    }

    let mut visible = Vec::new();
    for project_id in normalized_project_ids {
        let Some(project) = projects_by_id_map.get(&project_id) else {
            continue;
        };
        let root_id = project.root_project_id.unwrap_or(project.id);
        let Some(root) = roots_by_id.get(&root_id) else {
            continue;
        };
        let mut role = if root.user_id == user_id {
            Some(ProjectAccessRole::Owner)
        } else {
            member_roles.get(&root.id).copied()
        };
        if user.role == ROLE_READ_ONLY && role.is_some() {
            role = Some(ProjectAccessRole::Viewer);
        }
        if role.is_some_and(|role| role.satisfies(required)) {
            visible.push(project_id);
        }
    }
    Ok(visible)
}
