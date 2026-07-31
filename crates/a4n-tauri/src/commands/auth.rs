//! 用户认证命令。
//!
//! 保持 `commands::auth::*` 的 Tauri 命令入口稳定，认证业务规则由 runtime 统一承载。

use a4n_runtime::AppState;
use a4n_runtime::host_api::auth_records as auth_actions;
use a4n_runtime::host_api::oauth::{OAuth2AuthorizeInput, OAuth2CallbackInput};
use auth_actions::{
    AuthResult, ChangePasswordInput, LoginInput, OAuth2LoginResult, RefreshTokenInput,
    RegisterInput, SessionSummary, TokenPairSummary, UpdateProfileInput, UserSummary,
};
use tauri::State;

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

auth_command!(get_user(state, user_id: i64) -> UserSummary, auth_actions::get_user(state.inner(), user_id));
auth_command!(
    register_user(state, input: RegisterInput) -> AuthResult,
    auth_actions::register_user(state.inner(), input, Some("a4n-tauri"))
);
auth_command!(
    login_user(state, input: LoginInput) -> AuthResult,
    auth_actions::login_user(state.inner(), input, Some("a4n-tauri"))
);
auth_command!(
    update_user_profile(state, user_id: i64, input: UpdateProfileInput) -> UserSummary,
    auth_actions::update_user_profile_for_user(state.inner(), user_id, user_id, input)
);
auth_command!(
    change_user_password(state, user_id: i64, input: ChangePasswordInput) -> (),
    auth_actions::change_user_password_for_user(state.inner(), user_id, user_id, input)
);
auth_command!(
    logout_user(state, user_id: Option<i64>, session_id: Option<String>) -> (),
    auth_actions::logout_user(state.inner(), user_id, session_id.as_deref())
);
auth_command!(
    refresh_token(state, input: RefreshTokenInput) -> TokenPairSummary,
    auth_actions::refresh_token(state.inner(), input)
);
auth_command!(
    get_current_user(state, user_id: i64) -> UserSummary,
    auth_actions::get_current_user(state.inner(), user_id)
);
auth_command!(
    list_sessions(state, user_id: i64) -> Vec<SessionSummary>,
    auth_actions::list_sessions_for_user(state.inner(), user_id, user_id)
);
#[tauri::command]
pub fn oauth2_authorize_url(input: OAuth2AuthorizeInput) -> Result<String, String> {
    a4n_runtime::host_api::oauth::oauth2_authorize_url(input).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn oauth2_callback(
    state: State<'_, AppState>,
    input: OAuth2CallbackInput,
) -> Result<OAuth2LoginResult, String> {
    auth_actions::oauth2_callback_login(state.inner(), input, Some("a4n-tauri"))
        .await
        .map_err(|e| e.to_string())
}
