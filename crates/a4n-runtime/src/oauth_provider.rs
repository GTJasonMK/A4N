//! OAuth2 社交登录集成
//!
//! 支持 GitHub 和 Google 的 OAuth2 授权码流程。

use a4n_core::error::AppError;
use async_trait::async_trait;

mod contracts;
mod github;
mod google;

use contracts::urlencoding;
pub use contracts::{OAuth2Provider, OAuth2UserInfo};
use contracts::{oauth_token_error, required_non_empty_string};

pub use github::GitHubOAuth2;
pub use google::GoogleOAuth2;
