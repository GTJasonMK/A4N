//! JWT 令牌管理
//!
//! 基于 jsonwebtoken crate 实现访问令牌和刷新令牌的签发与验证。

use a4n_core::error::AppError;

mod contracts;
mod signing;
mod tokens;
mod verification;

use contracts::{ACCESS_TOKEN_EXP_MINUTES, ACCESS_TOKEN_EXPIRES_IN_SECS, REFRESH_TOKEN_EXP_DAYS};
pub use contracts::{Claims, TokenPair};
use signing::{create_access_token_with_role, create_refresh_token};
pub use tokens::{create_media_token, create_token_pair_with_role};
pub use verification::{refresh_access_token_with_role, verify_token};
