use crate::collaboration::{
    CollaborationDbError, INVITE_STATUS_PENDING, INVITE_TTL_DAYS, ProjectAccessRole,
    ProjectInviteSummary, ProjectMemberRole, expire_project_invites, find_user_by_identity,
    invite_summaries, resolve_root_project, verify_project_permission,
};
use a4n_core::models::project::{project_invite, project_member};
use chrono::{Duration, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set,
};

pub async fn invite_project_member(
    db: &DatabaseConnection,
    inviter_user_id: i64,
    project_id: i64,
    username_or_email: &str,
    role: ProjectMemberRole,
) -> Result<(), CollaborationDbError> {
    let inviter_role =
        verify_project_permission(db, project_id, inviter_user_id, ProjectAccessRole::Manager)
            .await?;
    if role == ProjectMemberRole::Manager && inviter_role != ProjectAccessRole::Owner {
        return Err(CollaborationDbError::Forbidden(
            "只有项目所有者可以邀请 manager".to_string(),
        ));
    }
    let root = resolve_root_project(db, project_id).await?;
    let invitee = find_user_by_identity(db, username_or_email).await?;
    if !invitee.is_active {
        return Err(CollaborationDbError::InvalidState(
            "被邀请用户已被禁用".to_string(),
        ));
    }
    if invitee.id == inviter_user_id {
        return Err(CollaborationDbError::InvalidArgument(
            "不能邀请自己".to_string(),
        ));
    }
    if invitee.id == root.user_id {
        return Err(CollaborationDbError::InvalidState(
            "项目所有者不需要邀请".to_string(),
        ));
    }
    if project_member::Entity::find()
        .select_only()
        .column(project_member::Column::Id)
        .filter(project_member::Column::ProjectId.eq(root.id))
        .filter(project_member::Column::UserId.eq(invitee.id))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .is_some()
    {
        return Err(CollaborationDbError::InvalidState(
            "该用户已是项目协作者".to_string(),
        ));
    }

    expire_project_invites(db, root.id).await?;
    if project_invite::Entity::find()
        .select_only()
        .column(project_invite::Column::Id)
        .filter(project_invite::Column::ProjectId.eq(root.id))
        .filter(project_invite::Column::InviteeUserId.eq(invitee.id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .is_some()
    {
        return Err(CollaborationDbError::InvalidState(
            "该用户已有待处理邀请".to_string(),
        ));
    }

    let now = Utc::now().naive_utc();
    project_invite::ActiveModel {
        project_id: Set(root.id),
        inviter_user_id: Set(inviter_user_id),
        invitee_user_id: Set(invitee.id),
        role: Set(role.as_str().to_string()),
        status: Set(INVITE_STATUS_PENDING.to_string()),
        expires_at: Set(now + Duration::days(INVITE_TTL_DAYS)),
        accepted_at: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(CollaborationDbError::Database)?;
    Ok(())
}

pub async fn list_project_invites(
    db: &DatabaseConnection,
    actor_user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectInviteSummary>, CollaborationDbError> {
    verify_project_permission(db, project_id, actor_user_id, ProjectAccessRole::Manager).await?;
    let root = resolve_root_project(db, project_id).await?;
    let invites = project_invite::Entity::find()
        .filter(project_invite::Column::ProjectId.eq(root.id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .filter(project_invite::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .order_by_desc(project_invite::Column::CreatedAt)
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    invite_summaries(db, invites).await
}

pub async fn list_my_project_invites(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<Vec<ProjectInviteSummary>, CollaborationDbError> {
    let invites = project_invite::Entity::find()
        .filter(project_invite::Column::InviteeUserId.eq(user_id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .filter(project_invite::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .order_by_desc(project_invite::Column::CreatedAt)
        .all(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    invite_summaries(db, invites).await
}
