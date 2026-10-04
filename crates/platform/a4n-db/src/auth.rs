use a4n_db_models::models::user::{invite_code, user, user_session};
use a4n_error::AppError;
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
use sha2::{Digest, Sha256};
use uuid::Uuid;

mod invites;
mod oauth;
mod rules;
mod sessions;
mod types;
mod users;

pub use a4n_domain::auth::UserSessionRecord;
pub(crate) use invites::register_user_with_invite;
pub use invites::{create_invite_code, list_invite_codes};
pub(crate) use oauth::login_with_oauth2;
pub use sessions::revoke_session;
#[cfg(test)]
pub(crate) use sessions::{
    create_session, list_sessions, rotate_session_refresh_token, verify_session_refresh_token,
};
pub use types::BootstrapAdminInput;
pub(crate) use users::lock_administrator_membership;
#[cfg(test)]
pub(crate) use users::{bootstrap_first_admin, disable_user, register_user};
pub use users::{
    change_password, current_user_summary, list_users, require_existing_user_or_bootstrap_admin,
    user_authorization_by_id, user_summary_by_id,
};
#[cfg(test)]
pub(crate) use users::{
    create_user, current_user, login_user, update_profile, update_user_role, user_by_id,
};

use a4n_domain::auth::UserSummary;

pub async fn create_user_summary(
    db: &impl crate::DbExecutor,
    username: &str,
    email: &str,
    password: &str,
    role: &str,
) -> Result<UserSummary, AppError> {
    users::create_user(db, username, email, password, role)
        .await
        .map(|user| users::user_summary_from_model(&user))
}

pub async fn update_user_role_summary(
    db: &impl crate::DbExecutor,
    user_id: i64,
    role: &str,
) -> Result<UserSummary, AppError> {
    users::update_user_role(db, user_id, role)
        .await
        .map(|user| users::user_summary_from_model(&user))
}

pub async fn register_user_with_invite_summary(
    db: &impl crate::DbExecutor,
    username: &str,
    email: &str,
    password: &str,
    invite_code: &str,
) -> Result<UserSummary, AppError> {
    register_user_with_invite(db, username, email, password, invite_code)
        .await
        .map(|user| users::user_summary_from_model(&user))
}

pub async fn login_user_summary(
    db: &impl crate::DbExecutor,
    username: &str,
    password: &str,
) -> Result<UserSummary, AppError> {
    users::login_user(db, username, password)
        .await
        .map(|user| users::user_summary_from_model(&user))
}

pub async fn update_profile_summary(
    db: &impl crate::DbExecutor,
    user_id: i64,
    email: Option<&str>,
    avatar_url: Option<Option<&str>>,
    page_cli_enabled: Option<bool>,
) -> Result<UserSummary, AppError> {
    users::update_profile(db, user_id, email, avatar_url, page_cli_enabled)
        .await
        .map(|user| users::user_summary_from_model(&user))
}

pub async fn login_with_oauth2_summary(
    db: &impl crate::DbExecutor,
    provider: &str,
    provider_id: &str,
    email: Option<&str>,
    username_hint: Option<&str>,
    avatar_url: Option<&str>,
) -> Result<UserSummary, AppError> {
    login_with_oauth2(db, provider, provider_id, email, username_hint, avatar_url)
        .await
        .map(|user| users::user_summary_from_model(&user))
}

pub async fn create_session_record(
    db: &impl crate::DbExecutor,
    user_id: i64,
    refresh_token: &str,
    user_agent: Option<&str>,
) -> Result<UserSessionRecord, AppError> {
    sessions::create_session(db, user_id, refresh_token, user_agent)
        .await
        .map(sessions::session_record)
}

pub async fn list_session_records(
    db: &impl crate::DbExecutor,
    user_id: i64,
) -> Result<Vec<UserSessionRecord>, AppError> {
    sessions::list_sessions(db, user_id)
        .await
        .map(|sessions| sessions.into_iter().map(sessions::session_record).collect())
}

pub async fn verify_session_refresh_token_valid(
    db: &impl crate::DbExecutor,
    user_id: i64,
    session_id: &str,
    refresh_token: &str,
) -> Result<(), AppError> {
    sessions::verify_session_refresh_token(db, user_id, session_id, refresh_token)
        .await
        .map(|_| ())
}

pub async fn verify_active_session_valid(
    db: &impl crate::DbExecutor,
    user_id: i64,
    session_id: &str,
) -> Result<(), AppError> {
    sessions::verify_active_session(db, user_id, session_id)
        .await
        .map(|_| ())
}

pub async fn rotate_session_refresh_token_for_user(
    db: &impl crate::DbExecutor,
    user_id: i64,
    session_id: &str,
    old_refresh_token: &str,
    new_refresh_token: &str,
) -> Result<(), AppError> {
    let session =
        sessions::verify_session_refresh_token(db, user_id, session_id, old_refresh_token).await?;
    sessions::rotate_session_refresh_token(db, session, old_refresh_token, new_refresh_token)
        .await
        .map(|_| ())
}

#[cfg(test)]
mod tests;
