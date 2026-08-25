use a4n_core::error::AppError;
use a4n_core::models::user::user;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

use crate::auth::rules::{
    hash_password, normalize_email, normalize_optional_text, normalize_username,
    validate_password_for_storage, verify_password,
};

use super::read::{active_user, find_by_email_on, find_by_username_on};

pub async fn login_user(
    db: &DatabaseConnection,
    username: &str,
    password: &str,
) -> Result<user::Model, AppError> {
    let username = normalize_username(username)?;
    let user = find_by_username_on(db, &username)
        .await?
        .ok_or_else(|| AppError::Unauthorized("用户名或密码错误".into()))?;

    if !user.is_active {
        return Err(AppError::Forbidden("账户已被禁用".into()));
    }
    if !verify_password(password, &user.password_hash)? {
        return Err(AppError::Unauthorized("用户名或密码错误".into()));
    }

    let mut active: user::ActiveModel = user.clone().into();
    active.last_login_at = Set(Some(Utc::now().naive_utc()));
    active.update(db).await.map_err(AppError::Database)
}

pub async fn update_profile(
    db: &DatabaseConnection,
    user_id: i64,
    email: Option<&str>,
    avatar_url: Option<Option<&str>>,
    page_cli_enabled: Option<bool>,
) -> Result<user::Model, AppError> {
    let user = active_user(db, user_id).await?;
    let mut active: user::ActiveModel = user.into();
    if let Some(value) = email {
        let email = normalize_email(value)?;
        if let Some(existing) = find_by_email_on(db, &email).await?
            && existing.id != user_id
        {
            return Err(AppError::InvalidArgument("邮箱已被注册".into()));
        }
        active.email = Set(email);
    }
    if let Some(value) = avatar_url {
        active.avatar_url = Set(value.and_then(normalize_optional_text));
    }
    if let Some(value) = page_cli_enabled {
        active.page_cli_enabled = Set(value);
    }
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)
}

pub async fn change_password(
    db: &DatabaseConnection,
    user_id: i64,
    old_password: &str,
    new_password: &str,
) -> Result<(), AppError> {
    let user = active_user(db, user_id).await?;
    if !verify_password(old_password, &user.password_hash)? {
        return Err(AppError::Unauthorized("原密码错误".into()));
    }
    validate_password_for_storage(new_password)?;
    let mut active: user::ActiveModel = user.into();
    active.password_hash = Set(hash_password(new_password)?);
    active.updated_at = Set(Utc::now().naive_utc());
    active.update(db).await.map_err(AppError::Database)?;
    Ok(())
}
