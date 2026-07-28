pub use a4n_api_contract::auth::{
    AuthResult, ChangePasswordInput, LoginInput, OAuth2LoginResult, RefreshTokenInput,
    RegisterInput, SessionSummary, TokenPairSummary, UpdateProfileInput, UserSummary,
};

#[derive(Debug, Clone, Copy)]
pub struct BootstrapAdminInput<'a> {
    pub username: &'a str,
    pub email: &'a str,
    pub password: &'a str,
}
