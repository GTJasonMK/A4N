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

pub use invites::*;
pub use oauth::*;
pub use sessions::*;
pub use types::*;
pub use users::*;

#[cfg(test)]
mod tests;
