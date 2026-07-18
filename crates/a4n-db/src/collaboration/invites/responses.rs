use crate::collaboration::{
    CollaborationDbError, INVITE_STATUS_ACCEPTED, INVITE_STATUS_DECLINED, INVITE_STATUS_EXPIRED,
    INVITE_STATUS_PENDING, INVITE_STATUS_REVOKED, ProjectAccessRole, ProjectInviteSummary,
    ProjectMemberSummary, find_invite, find_project, invite_summary, member_summary,
    resolve_root_project_model, update_invite_status, verify_project_permission,
};
use a4n_core::models::project::{project_invite, project_member};
use chrono::Utc;
use sea_orm::sea_query::{Expr, OnConflict};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};

pub async fn accept_project_invite(
    db: &DatabaseConnection,
    user_id: i64,
    invite_id: i64,
) -> Result<ProjectMemberSummary, CollaborationDbError> {
    let tx = db.begin().await.map_err(CollaborationDbError::Database)?;
    let invite = find_invite(&tx, invite_id).await?;
    if invite.invitee_user_id != user_id {
        return Err(CollaborationDbError::Forbidden(
            "只能处理自己的项目邀请".to_string(),
        ));
    }
    if invite.status != INVITE_STATUS_PENDING {
        return Err(CollaborationDbError::InvalidState(
            "邀请已不可接受".to_string(),
        ));
    }
    let now = Utc::now().naive_utc();
    if invite.expires_at <= now {
        project_invite::Entity::update_many()
            .col_expr(
                project_invite::Column::Status,
                Expr::value(INVITE_STATUS_EXPIRED),
            )
            .col_expr(project_invite::Column::UpdatedAt, Expr::value(now))
            .filter(project_invite::Column::Id.eq(invite.id))
            .exec(&tx)
            .await
            .map_err(CollaborationDbError::Database)?;
        tx.commit().await.map_err(CollaborationDbError::Database)?;
        return Err(CollaborationDbError::InvalidState("邀请已过期".to_string()));
    }
    let root = find_project(&tx, invite.project_id).await?;
    if root.user_id == user_id {
        return Err(CollaborationDbError::InvalidState(
            "项目所有者不需要接受邀请".to_string(),
        ));
    }

    let member_active = project_member::ActiveModel {
        project_id: Set(invite.project_id),
        user_id: Set(user_id),
        role: Set(invite.role.clone()),
        invited_by_user_id: Set(invite.inviter_user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    project_member::Entity::insert(member_active)
        .on_conflict(
            OnConflict::columns([
                project_member::Column::ProjectId,
                project_member::Column::UserId,
            ])
            .update_columns([
                project_member::Column::Role,
                project_member::Column::InvitedByUserId,
                project_member::Column::UpdatedAt,
            ])
            .to_owned(),
        )
        .exec(&tx)
        .await
        .map_err(CollaborationDbError::Database)?;
    project_invite::Entity::update_many()
        .col_expr(
            project_invite::Column::Status,
            Expr::value(INVITE_STATUS_ACCEPTED),
        )
        .col_expr(project_invite::Column::AcceptedAt, Expr::value(Some(now)))
        .col_expr(project_invite::Column::UpdatedAt, Expr::value(now))
        .filter(project_invite::Column::Id.eq(invite.id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .exec(&tx)
        .await
        .map_err(CollaborationDbError::Database)?;
    tx.commit().await.map_err(CollaborationDbError::Database)?;

    let member = project_member::Entity::find()
        .filter(project_member::Column::ProjectId.eq(invite.project_id))
        .filter(project_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(CollaborationDbError::Database)?
        .ok_or_else(|| CollaborationDbError::Internal("协作成员写入后未找到".to_string()))?;
    member_summary(db, member).await
}

pub async fn decline_project_invite(
    db: &DatabaseConnection,
    user_id: i64,
    invite_id: i64,
) -> Result<ProjectInviteSummary, CollaborationDbError> {
    let invite = find_invite(db, invite_id).await?;
    if invite.invitee_user_id != user_id {
        return Err(CollaborationDbError::Forbidden(
            "只能处理自己的项目邀请".to_string(),
        ));
    }
    let updated = update_invite_status(db, invite_id, INVITE_STATUS_DECLINED, None).await?;
    invite_summary(db, updated).await
}

pub async fn revoke_project_invite(
    db: &DatabaseConnection,
    actor_user_id: i64,
    project_id: i64,
    invite_id: i64,
) -> Result<ProjectInviteSummary, CollaborationDbError> {
    verify_project_permission(db, project_id, actor_user_id, ProjectAccessRole::Manager).await?;
    let root = resolve_root_project_model(db, project_id).await?;
    let invite = find_invite(db, invite_id).await?;
    if invite.project_id != root.id {
        return Err(CollaborationDbError::Forbidden(
            "邀请不属于该项目".to_string(),
        ));
    }
    let updated = update_invite_status(db, invite_id, INVITE_STATUS_REVOKED, None).await?;
    invite_summary(db, updated).await
}
