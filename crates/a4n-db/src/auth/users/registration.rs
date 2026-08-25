use a4n_core::error::AppError;
#[cfg(test)]
use a4n_core::models::user::role::ROLE_USER;
use a4n_core::models::user::{
    role::{ROLE_ADMIN, validate_user_role},
    user,
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DatabaseConnection, Set};

use crate::auth::BootstrapAdminInput;

use super::read::{count_users, find_by_email_on, find_by_username_on};
use crate::auth::rules::{
    hash_password, normalize_email, normalize_username, validate_password_for_storage,
};

#[cfg(test)]
pub async fn register_user(
    db: &DatabaseConnection,
    username: &str,
    email: &str,
    password: &str,
) -> Result<user::Model, AppError> {
    create_user(db, username, email, password, ROLE_USER).await
}

pub async fn create_user(
    db: &DatabaseConnection,
    username: &str,
    email: &str,
    password: &str,
    role: &str,
) -> Result<user::Model, AppError> {
    create_user_on(db, username, email, password, role).await
}

pub async fn bootstrap_first_admin(
    db: &DatabaseConnection,
    username: &str,
    email: &str,
    password: &str,
) -> Result<Option<user::Model>, AppError> {
    if count_users(db).await? != 0 {
        return Ok(None);
    }
    create_user(db, username, email, password, ROLE_ADMIN)
        .await
        .map(Some)
}

pub async fn require_existing_user_or_bootstrap_admin(
    db: &DatabaseConnection,
    bootstrap: Option<BootstrapAdminInput<'_>>,
) -> Result<(), AppError> {
    if count_users(db).await? != 0 {
        return Ok(());
    }
    let input = bootstrap.ok_or_else(|| {
        AppError::ConfigError(
            "用户表为空，Web 部署必须设置 A4N_BOOTSTRAP_ADMIN_USERNAME、A4N_BOOTSTRAP_ADMIN_EMAIL、A4N_BOOTSTRAP_ADMIN_PASSWORD 创建首个管理员"
                .into(),
        )
    })?;
    bootstrap_first_admin(db, input.username, input.email, input.password).await?;
    Ok(())
}

pub(in crate::auth) async fn create_user_on<C>(
    db: &C,
    username: &str,
    email: &str,
    password: &str,
    role: &str,
) -> Result<user::Model, AppError>
where
    C: ConnectionTrait,
{
    let username = normalize_username(username)?;
    let email = normalize_email(email)?;
    validate_user_role(role)?;
    if find_by_username_on(db, &username).await?.is_some() {
        return Err(AppError::InvalidArgument("用户名已被占用".into()));
    }
    if find_by_email_on(db, &email).await?.is_some() {
        return Err(AppError::InvalidArgument("邮箱已被注册".into()));
    }
    validate_password_for_storage(password)?;

    let now = Utc::now().naive_utc();
    user::ActiveModel {
        username: Set(username),
        email: Set(email),
        password_hash: Set(hash_password(password)?),
        role: Set(role.to_string()),
        oauth_provider: Set(None),
        oauth_subject: Set(None),
        avatar_url: Set(None),
        page_cli_enabled: Set(false),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)
}
