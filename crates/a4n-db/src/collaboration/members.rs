use super::*;

pub async fn list_project_members(
    db: &DatabaseConnection,
    actor_user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectMemberSummary>, CollaborationDbError> {
    verify_project_permission(db, project_id, actor_user_id, ProjectAccessRole::Viewer).await?;
    let root = resolve_root_project_model(db, project_id).await?;
    let rows = project_member::Entity::find()
        .filter(project_member::Column::ProjectId.eq(root.id))
        .order_by_asc(project_member::Column::CreatedAt)
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    let user_ids = unique_member_user_ids(root.user_id, &rows);
    let users_by_id = users_by_id(db, &user_ids).await?;
    let owner = users_by_id
        .get(&root.user_id)
        .ok_or_else(|| CollaborationDbError::Internal(format!("用户不存在: {}", root.user_id)))?;
    let mut members = vec![ProjectMemberSummary {
        project_id: root.id,
        user_id: owner.id,
        username: owner.username.clone(),
        email: owner.email.clone(),
        role: ProjectAccessRole::Owner.as_str().to_string(),
        is_owner: true,
        invited_by_user_id: None,
        created_at: Some(root.created_at.to_string()),
        updated_at: Some(root.updated_at.to_string()),
    }];

    for row in rows {
        if let Some(user) = users_by_id.get(&row.user_id) {
            members.push(ProjectMemberSummary {
                project_id: root.id,
                user_id: row.user_id,
                username: user.username.clone(),
                email: user.email.clone(),
                role: row.role,
                is_owner: false,
                invited_by_user_id: Some(row.invited_by_user_id),
                created_at: Some(row.created_at.to_string()),
                updated_at: Some(row.updated_at.to_string()),
            });
        }
    }
    Ok(members)
}

pub async fn update_project_member_role(
    db: &DatabaseConnection,
    actor_user_id: i64,
    project_id: i64,
    member_user_id: i64,
    role: ProjectMemberRole,
) -> Result<ProjectMemberSummary, CollaborationDbError> {
    let actor_role =
        verify_project_permission(db, project_id, actor_user_id, ProjectAccessRole::Manager)
            .await?;
    let root = resolve_root_project_model(db, project_id).await?;
    if member_user_id == root.user_id {
        return Err(CollaborationDbError::InvalidState(
            "不能修改项目所有者角色".to_string(),
        ));
    }
    let member = find_member(db, root.id, member_user_id).await?;
    let current_role = parse_project_member_role(&member.role)?;
    if actor_role != ProjectAccessRole::Owner
        && (current_role == ProjectMemberRole::Manager || role == ProjectMemberRole::Manager)
    {
        return Err(CollaborationDbError::Forbidden(
            "manager 不能管理其他 manager 或提升 manager".to_string(),
        ));
    }
    let now = Utc::now().naive_utc();
    let mut active: project_member::ActiveModel = member.into();
    active.role = Set(role.as_str().to_string());
    active.updated_at = Set(now);
    let updated = active
        .update(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    member_summary(db, updated).await
}

pub async fn remove_project_member(
    db: &DatabaseConnection,
    actor_user_id: i64,
    project_id: i64,
    member_user_id: i64,
) -> Result<(), CollaborationDbError> {
    let actor_role =
        verify_project_permission(db, project_id, actor_user_id, ProjectAccessRole::Manager)
            .await?;
    let root = resolve_root_project_model(db, project_id).await?;
    if member_user_id == root.user_id {
        return Err(CollaborationDbError::InvalidState(
            "不能移除项目所有者".to_string(),
        ));
    }
    let member = find_member(db, root.id, member_user_id).await?;
    let current_role = parse_project_member_role(&member.role)?;
    if actor_role != ProjectAccessRole::Owner && current_role == ProjectMemberRole::Manager {
        return Err(CollaborationDbError::Forbidden(
            "manager 不能移除其他 manager".to_string(),
        ));
    }
    project_member::Entity::delete_by_id(member.id)
        .exec(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(())
}

pub async fn leave_project(
    db: &DatabaseConnection,
    user_id: i64,
    project_id: i64,
) -> Result<(), CollaborationDbError> {
    let root = resolve_root_project_model(db, project_id).await?;
    if root.user_id == user_id {
        return Err(CollaborationDbError::InvalidState(
            "项目所有者不能离开自己的项目".to_string(),
        ));
    }
    project_member::Entity::delete_many()
        .filter(project_member::Column::ProjectId.eq(root.id))
        .filter(project_member::Column::UserId.eq(user_id))
        .exec(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(())
}
