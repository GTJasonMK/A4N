//! OAuth2 社交登录集成
//!
//! 支持 GitHub 和 Google 的 OAuth2 授权码流程。

mod contracts;
mod github;
mod google;

pub use contracts::OAuth2Provider;

pub use github::GitHubOAuth2;
pub use google::GoogleOAuth2;
