use a4n_core::error::AppError;
use a4n_core::models::user::{invite_code, role::ROLE_USER, user, user_session};
use argon2::password_hash::{SaltString, rand_core::OsRng};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use chrono::{Duration, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
    TransactionTrait,
};
use sha2::{Digest, Sha256};
use uuid::Uuid;

mod invites;
mod oauth;
mod rules;
mod sessions;
mod types;
mod users;

pub use invites::{create_invite_code, list_invite_codes, register_user_with_invite};
pub use oauth::login_with_oauth2;
pub use sessions::{
    create_session, list_sessions, revoke_session, rotate_session_refresh_token,
    verify_active_session, verify_session_refresh_token,
};
pub use types::BootstrapAdminInput;
#[cfg(test)]
pub use users::{bootstrap_first_admin, disable_user, register_user};
pub use users::{
    change_password, create_user, current_user, list_users, login_user,
    require_existing_user_or_bootstrap_admin, update_profile, update_user_role, user_by_id,
};

#[cfg(test)]
mod tests;
