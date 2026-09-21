use crate::collaboration::{
    CollaborationDbError, INVITE_STATUS_ACCEPTED, INVITE_STATUS_DECLINED, INVITE_STATUS_EXPIRED,
    INVITE_STATUS_PENDING, INVITE_STATUS_REVOKED, ProjectAccessRole, find_invite, find_project,
    parse_stored_member_role, resolve_root_project, update_pending_invite_status,
    verify_project_permission,
};
use a4n_db_models::models::project::{project_invite, project_member};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set, TransactionTrait};

pub async fn accept_project_invite(
    db: &DatabaseConnection,
    user_id: i64,
    invite_id: i64,
) -> Result<(), CollaborationDbError> {
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
        let result = project_invite::Entity::update_many()
            .col_expr(
                project_invite::Column::Status,
                Expr::value(INVITE_STATUS_EXPIRED),
            )
            .col_expr(project_invite::Column::UpdatedAt, Expr::value(now))
            .filter(project_invite::Column::Id.eq(invite.id))
            .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
            .exec(&tx)
            .await
            .map_err(CollaborationDbError::Database)?;
        if result.rows_affected != 1 {
            return Err(CollaborationDbError::InvalidState(
                "邀请已不可接受".to_string(),
            ));
        }
        tx.commit().await.map_err(CollaborationDbError::Database)?;
        return Err(CollaborationDbError::InvalidState("邀请已过期".to_string()));
    }
    let root = find_project(&tx, invite.project_id).await?;
    if root.1 == user_id {
        return Err(CollaborationDbError::InvalidState(
            "项目所有者不需要接受邀请".to_string(),
        ));
    }
    let role = parse_stored_member_role(&invite.role)?;

    let member_active = project_member::ActiveModel {
        project_id: Set(invite.project_id),
        user_id: Set(user_id),
        role: Set(role.as_str().to_string()),
        invited_by_user_id: Set(invite.inviter_user_id),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    project_member::Entity::insert(member_active)
        .exec(&tx)
        .await
        .map_err(CollaborationDbError::Database)?;
    let result = project_invite::Entity::update_many()
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
    if result.rows_affected != 1 {
        return Err(CollaborationDbError::InvalidState(
            "邀请已不可接受".to_string(),
        ));
    }
    tx.commit().await.map_err(CollaborationDbError::Database)?;
    Ok(())
}

pub async fn decline_project_invite(
    db: &DatabaseConnection,
    user_id: i64,
    invite_id: i64,
) -> Result<(), CollaborationDbError> {
    let invite = find_invite(db, invite_id).await?;
    if invite.invitee_user_id != user_id {
        return Err(CollaborationDbError::Forbidden(
            "只能处理自己的项目邀请".to_string(),
        ));
    }
    update_pending_invite_status(db, invite_id, INVITE_STATUS_DECLINED).await
}

pub async fn revoke_project_invite(
    db: &DatabaseConnection,
    actor_user_id: i64,
    project_id: i64,
    invite_id: i64,
) -> Result<(), CollaborationDbError> {
    verify_project_permission(db, project_id, actor_user_id, ProjectAccessRole::Manager).await?;
    let (root_result, invite_result) = futures::join!(
        resolve_root_project(db, project_id),
        find_invite(db, invite_id),
    );
    let root = root_result?;
    let invite = invite_result?;
    if invite.project_id != root.id {
        return Err(CollaborationDbError::Forbidden(
            "邀请不属于该项目".to_string(),
        ));
    }
    update_pending_invite_status(db, invite_id, INVITE_STATUS_REVOKED).await
}
