use a4n_db_models::models::user::user;
use a4n_domain::auth::{ROLE_ADMIN, validate_user_role};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseTransaction, DbBackend, EntityTrait,
    PaginatorTrait, QueryFilter, QuerySelect, Set, Statement,
};

use crate::auth::rules::validate_positive_user_id;

pub async fn update_user_role(
    db: &impl crate::DbExecutor,
    user_id: i64,
    role: &str,
) -> Result<user::Model, AppError> {
    validate_positive_user_id(user_id)?;
    validate_user_role(role)?;

    let tx = db.begin().await.map_err(AppError::from)?;
    let user = lock_administrator_membership(&tx, user_id, role != ROLE_ADMIN).await?;
    let mut active: user::ActiveModel = user.into();
    active.role = Set(role.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    let updated = active.update(&tx).await.map_err(AppError::from)?;
    tx.commit().await.map_err(AppError::from)?;
    Ok(updated)
}

pub(crate) async fn lock_administrator_membership(
    tx: &DatabaseTransaction,
    user_id: i64,
    removes_access: bool,
) -> Result<user::Model, AppError> {
    // ponytail: administrator membership changes are serialized per instance;
    // use a tenant lock if administrator membership becomes tenant-scoped.
    tx.query_one(Statement::from_string(
        DbBackend::Postgres,
        "SELECT pg_advisory_xact_lock(hashtextextended('a4n-auth-admin-membership', 0))",
    ))
    .await
    .map_err(AppError::from)?;
    let user = user::Entity::find_by_id(user_id)
        .lock_exclusive()
        .one(tx)
        .await
        .map_err(AppError::from)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))?;
    if removes_access
        && user.role == ROLE_ADMIN
        && user.is_active
        && user.lifecycle_gate == crate::generation::USER_GATE_ACTIVE
    {
        let others = user::Entity::find()
            .filter(user::Column::Id.ne(user_id))
            .filter(user::Column::Role.eq(ROLE_ADMIN))
            .filter(user::Column::IsActive.eq(true))
            .filter(user::Column::LifecycleGate.eq(crate::generation::USER_GATE_ACTIVE))
            .count(tx)
            .await
            .map_err(AppError::from)?;
        if others == 0 {
            return Err(AppError::InvalidArgument(
                "必须保留至少一位可用的管理员，请先将其他用户设为管理员".into(),
            ));
        }
    }
    Ok(user)
}

#[cfg(test)]
pub async fn disable_user(
    db: &impl crate::DbExecutor,
    caller_user_id: i64,
    user_id: i64,
) -> Result<(), AppError> {
    validate_positive_user_id(user_id)?;
    if caller_user_id == user_id {
        return Err(AppError::InvalidArgument("不能禁用当前登录用户".into()));
    }

    crate::generation::complete_user_disable_gate(db, user_id)
        .await
        .map_err(|error| AppError::InvalidState {
            current: error.to_string(),
            operation: "完成用户禁用 lifecycle gate".to_string(),
        })
}
