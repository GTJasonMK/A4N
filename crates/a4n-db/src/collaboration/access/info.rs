use std::collections::HashMap;

use a4n_core::models::{project::project_member, user::role::ROLE_READ_ONLY};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

use super::super::{
    ACCESS_ROLE_OWNER, CollaborationDbError, MEMBER_ROLE_VIEWER, ProjectAccessInfo,
    ProjectAccessInfoQuery, find_user, member_role_to_access_role, pending_invite_counts_by_root,
    projects_by_id, role_satisfies_manager, users_by_id, validate_collaboration_root,
    validate_positive_i64,
};

pub async fn project_access_info(
    db: &DatabaseConnection,
    query: ProjectAccessInfoQuery,
    user_id: i64,
) -> Result<ProjectAccessInfo, CollaborationDbError> {
    let mut infos = project_access_info_batch(db, std::slice::from_ref(&query), user_id).await?;
    infos.remove(&query.project_id).ok_or_else(|| {
        CollaborationDbError::Internal(format!("项目缺少协作访问信息: {}", query.project_id))
    })
}

pub async fn project_access_info_batch(
    db: &DatabaseConnection,
    queries: &[ProjectAccessInfoQuery],
    user_id: i64,
) -> Result<HashMap<i64, ProjectAccessInfo>, CollaborationDbError> {
    if queries.is_empty() {
        return Ok(HashMap::new());
    }

    let user = find_user(db, user_id).await?;
    if !user.is_active {
        return Err(CollaborationDbError::Forbidden("账户已被禁用".to_string()));
    }

    let mut root_ids = Vec::with_capacity(queries.len());
    for query in queries {
        validate_positive_i64(query.project_id, "project_id")?;
        root_ids.push(query.root_project_id.unwrap_or(query.project_id));
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

    let members = project_member::Entity::find()
        .filter(project_member::Column::ProjectId.is_in(root_ids.iter().copied()))
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    let mut member_counts: HashMap<i64, u64> = HashMap::new();
    let mut member_roles: HashMap<i64, String> = HashMap::new();
    for member in members {
        *member_counts.entry(member.project_id).or_insert(0) += 1;
        if member.user_id == user_id {
            member_roles.insert(member.project_id, member_role_to_access_role(&member.role)?);
        }
    }

    let owner_ids = roots_by_id
        .values()
        .map(|root| root.user_id)
        .collect::<Vec<_>>();
    let owners_by_id = users_by_id(db, &owner_ids).await?;

    let mut role_by_root = HashMap::with_capacity(root_ids.len());
    let mut manager_root_ids = Vec::new();
    for root_id in &root_ids {
        let root = roots_by_id.get(root_id).expect("root was validated above");
        let mut role = if root.user_id == user_id {
            Some(ACCESS_ROLE_OWNER.to_string())
        } else {
            member_roles.get(root_id).cloned()
        };
        if user.role == ROLE_READ_ONLY && role.is_some() {
            role = Some(MEMBER_ROLE_VIEWER.to_string());
        }
        if role.as_deref().map(role_satisfies_manager).unwrap_or(false) {
            manager_root_ids.push(*root_id);
        }
        role_by_root.insert(*root_id, role);
    }

    let pending_invite_counts = pending_invite_counts_by_root(db, &manager_root_ids).await?;
    let mut infos = HashMap::with_capacity(queries.len());
    for query in queries {
        let root_id = query.root_project_id.unwrap_or(query.project_id);
        let root = roots_by_id.get(&root_id).expect("root was validated above");
        let role = role_by_root.get(&root_id).cloned().flatten();
        infos.insert(
            query.project_id,
            ProjectAccessInfo {
                collaboration_root_project_id: root.id,
                owner_user_id: root.user_id,
                owner_username: owners_by_id
                    .get(&root.user_id)
                    .map(|owner| owner.username.clone()),
                my_role: role.clone(),
                member_count: member_counts.get(&root.id).copied().unwrap_or(0),
                pending_invite_count: if role
                    .as_deref()
                    .map(role_satisfies_manager)
                    .unwrap_or(false)
                {
                    Some(pending_invite_counts.get(&root.id).copied().unwrap_or(0))
                } else {
                    None
                },
            },
        );
    }
    Ok(infos)
}
