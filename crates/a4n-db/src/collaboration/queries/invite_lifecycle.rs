use super::lookups::find_invite;
use crate::collaboration::CollaborationDbError;
use crate::collaboration::types::{INVITE_STATUS_EXPIRED, INVITE_STATUS_PENDING};
use a4n_core::models::project::project_invite;
use chrono::{NaiveDateTime, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};

pub(in crate::collaboration) async fn expire_project_invites(
    db: &DatabaseConnection,
    project_id: i64,
) -> Result<(), CollaborationDbError> {
    project_invite::Entity::update_many()
        .col_expr(
            project_invite::Column::Status,
            Expr::value(INVITE_STATUS_EXPIRED),
        )
        .col_expr(
            project_invite::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(project_invite::Column::ProjectId.eq(project_id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .filter(project_invite::Column::ExpiresAt.lte(Utc::now().naive_utc()))
        .exec(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(())
}

pub(in crate::collaboration) async fn expire_user_invites(
    db: &DatabaseConnection,
    user_id: i64,
) -> Result<(), CollaborationDbError> {
    project_invite::Entity::update_many()
        .col_expr(
            project_invite::Column::Status,
            Expr::value(INVITE_STATUS_EXPIRED),
        )
        .col_expr(
            project_invite::Column::UpdatedAt,
            Expr::value(Utc::now().naive_utc()),
        )
        .filter(project_invite::Column::InviteeUserId.eq(user_id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .filter(project_invite::Column::ExpiresAt.lte(Utc::now().naive_utc()))
        .exec(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    Ok(())
}

pub(in crate::collaboration) async fn update_invite_status(
    db: &DatabaseConnection,
    invite_id: i64,
    status: &str,
    accepted_at: Option<NaiveDateTime>,
) -> Result<project_invite::Model, CollaborationDbError> {
    let invite = find_invite(db, invite_id).await?;
    if invite.status != INVITE_STATUS_PENDING {
        return Err(CollaborationDbError::InvalidState(
            "邀请已不可处理".to_string(),
        ));
    }
    let now = Utc::now().naive_utc();
    let mut active: project_invite::ActiveModel = invite.into();
    active.status = Set(status.to_string());
    active.accepted_at = Set(accepted_at);
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map_err(CollaborationDbError::Database)
}
