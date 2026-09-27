use axum::http::HeaderMap;
use serde_json::{Value, json};

use super::super::{
    ApiError, ServerState, auth_records_actions, command_input, oauth_actions, to_value, user_agent,
};

pub(in crate::routes::commands) async fn login_user(
    state: &ServerState,
    headers: &HeaderMap,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: auth_records_actions::LoginInput = command_input(args, "login_user")?;
    to_value(
        auth_records_actions::login_user(state.runtime(), input, user_agent(headers).as_deref())
            .await?,
    )
}

pub(in crate::routes::commands) async fn register_user(
    state: &ServerState,
    headers: &HeaderMap,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: auth_records_actions::RegisterInput = command_input(args, "register_user")?;
    to_value(
        auth_records_actions::register_user(state.runtime(), input, user_agent(headers).as_deref())
            .await?,
    )
}

pub(in crate::routes::commands) async fn refresh_token(
    state: &ServerState,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: auth_records_actions::RefreshTokenInput = command_input(args, "refresh_token")?;
    to_value(auth_records_actions::refresh_token(state.runtime(), input).await?)
}

pub(in crate::routes::commands) async fn oauth2_authorize_url(
    _state: &ServerState,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2AuthorizeInput = command_input(args, "oauth2_authorize_url")?;
    Ok(json!(oauth_actions::oauth2_authorize_url(input)?))
}

pub(in crate::routes::commands) async fn oauth2_available_providers(
    _state: &ServerState,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2AvailableProvidersInput =
        command_input(args, "oauth2_available_providers")?;
    Ok(json!(oauth_actions::oauth2_available_providers(input)?))
}

pub(in crate::routes::commands) async fn oauth2_callback(
    state: &ServerState,
    headers: &HeaderMap,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2CallbackInput = command_input(args, "oauth2_callback")?;
    to_value(
        auth_records_actions::oauth2_callback_login(
            state.runtime(),
            input,
            user_agent(headers).as_deref(),
        )
        .await?,
    )
}

pub(in crate::routes::commands) async fn get_current_user(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    to_value(auth_records_actions::get_current_user(state.runtime(), user_id).await?)
}

pub(in crate::routes::commands) async fn update_user_profile(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: auth_records_actions::UpdateProfileInput =
        command_input(args, "update_user_profile")?;
    to_value(auth_records_actions::update_user_profile(state.runtime(), user_id, input).await?)
}

pub(in crate::routes::commands) async fn change_user_password(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: auth_records_actions::ChangePasswordInput =
        command_input(args, "change_user_password")?;
    auth_records_actions::change_user_password(state.runtime(), user_id, input).await?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn list_sessions(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    to_value(auth_records_actions::list_sessions(state.runtime(), user_id).await?)
}

pub(in crate::routes::commands) async fn revoke_session(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: auth_records_actions::RevokeSessionInput = command_input(args, "revoke_session")?;
    auth_records_actions::revoke_session(state.runtime(), user_id, &input.session_id).await?;
    Ok(Value::Null)
}
