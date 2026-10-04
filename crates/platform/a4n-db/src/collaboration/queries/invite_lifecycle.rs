use crate::collaboration::CollaborationDbError;
use crate::collaboration::types::{INVITE_STATUS_EXPIRED, INVITE_STATUS_PENDING};
use a4n_db_models::models::project::project_invite;
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};

pub(in crate::collaboration) async fn expire_project_invites(
    db: &impl crate::DbExecutor,
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

pub(in crate::collaboration) async fn update_pending_invite_status(
    db: &impl crate::DbExecutor,
    invite_id: i64,
    status: &str,
) -> Result<(), CollaborationDbError> {
    let now = Utc::now().naive_utc();
    let result = project_invite::Entity::update_many()
        .col_expr(project_invite::Column::Status, Expr::value(status))
        .col_expr(project_invite::Column::UpdatedAt, Expr::value(now))
        .filter(project_invite::Column::Id.eq(invite_id))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .exec(db)
        .await
        .map_err(CollaborationDbError::Database)?;
    if result.rows_affected != 1 {
        return Err(CollaborationDbError::InvalidState(
            "邀请已不可处理".to_string(),
        ));
    }
    Ok(())
}
