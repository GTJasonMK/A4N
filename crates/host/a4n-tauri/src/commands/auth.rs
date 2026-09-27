//! 用户认证命令。
//!
//! 登录与会话等原生认证命令；其余已认证业务统一走 Host dispatcher。

use a4n_runtime::AppState;
use a4n_runtime::host_api::auth_records as auth_actions;
use a4n_runtime::host_api::oauth::{
    OAuth2AuthorizeInput, OAuth2AvailableProvidersInput, OAuth2CallbackInput,
};
use auth_actions::{AuthResult, LoginInput, RefreshTokenInput, RegisterInput, TokenPairSummary};
use tauri::State;

pub(crate) mod browser;
pub(crate) use browser::OAuthBrowserState;

macro_rules! auth_command {
    ($name:ident($state:ident $(, $arg:ident: $ty:ty)*) -> $output:ty, $body:expr) => {
        #[tauri::command]
        pub async fn $name(
            $state: State<'_, AppState>,
            $($arg: $ty,)*
        ) -> Result<$output, String> {
            $body.await.map_err(|e| e.to_string())
        }
    };
}

auth_command!(
    register_user(state, input: RegisterInput) -> AuthResult,
    auth_actions::register_user(state.inner(), input, Some("a4n-tauri"))
);
auth_command!(
    login_user(state, input: LoginInput) -> AuthResult,
    auth_actions::login_user(state.inner(), input, Some("a4n-tauri"))
);
auth_command!(
    refresh_token(state, input: RefreshTokenInput) -> TokenPairSummary,
    auth_actions::refresh_token(state.inner(), input)
);
#[tauri::command]
pub fn oauth2_authorize_url(input: OAuth2AuthorizeInput) -> Result<String, String> {
    a4n_runtime::host_api::oauth::oauth2_authorize_url(input).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn oauth2_available_providers(
    input: OAuth2AvailableProvidersInput,
) -> Result<Vec<String>, String> {
    a4n_runtime::host_api::oauth::oauth2_available_providers(input).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn oauth2_callback(
    state: State<'_, AppState>,
    input: OAuth2CallbackInput,
) -> Result<AuthResult, String> {
    auth_actions::oauth2_callback_login(state.inner(), input, Some("a4n-tauri"))
        .await
        .map_err(|e| e.to_string())
}
