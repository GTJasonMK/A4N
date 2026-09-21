use a4n_db_models::models::user::{role::validate_user_role, user};
use a4n_error::AppError;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use crate::auth::rules::validate_positive_user_id;

use super::read::user_by_id;

pub async fn update_user_role(
    db: &DatabaseConnection,
    user_id: i64,
    role: &str,
) -> Result<user::Model, AppError> {
    validate_positive_user_id(user_id)?;
    validate_user_role(role)?;

    let user = user_by_id(db, user_id).await?;
    let mut active: user::ActiveModel = user.into();
    active.role = Set(role.to_string());
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

#[cfg(test)]
pub async fn disable_user(
    db: &DatabaseConnection,
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
