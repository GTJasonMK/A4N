use axum::http::HeaderMap;
use serde_json::{Value, json};

use super::super::{
    ApiError, ServerState, auth_records_actions, command_input, i64_arg, issue_auth_response,
    oauth_actions, string_arg, to_value,
};

pub(in crate::routes::commands) async fn oauth2_authorize_url(
    _state: &ServerState,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2AuthorizeInput = command_input(args, "oauth2_authorize_url")?;
    Ok(json!(oauth_actions::oauth2_authorize_url(input)?))
}

pub(in crate::routes::commands) async fn oauth2_callback(
    state: &ServerState,
    headers: &HeaderMap,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: oauth_actions::OAuth2CallbackInput = command_input(args, "oauth2_callback")?;
    let user = auth_records_actions::oauth2_callback_user(state.runtime(), input).await?;
    let response = issue_auth_response(state, headers, user).await?;
    to_value(response)
}

pub(in crate::routes::commands) async fn get_user(
    state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let user_id = i64_arg(args, "userId")?;
    to_value(auth_records_actions::get_user(state.runtime(), user_id).await?)
}

pub(in crate::routes::commands) async fn update_user_profile(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    let input: auth_records_actions::UpdateProfileInput =
        command_input(args, "update_user_profile")?;
    to_value(
        auth_records_actions::update_user_profile_for_user(
            state.runtime(),
            user_id,
            target_user_id,
            input,
        )
        .await?,
    )
}

pub(in crate::routes::commands) async fn change_user_password(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    let input: auth_records_actions::ChangePasswordInput =
        command_input(args, "change_user_password")?;
    auth_records_actions::change_user_password_for_user(
        state.runtime(),
        user_id,
        target_user_id,
        input,
    )
    .await?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn list_sessions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    to_value(
        auth_records_actions::list_sessions_for_user(state.runtime(), user_id, target_user_id)
            .await?,
    )
}

pub(in crate::routes::commands) async fn revoke_session(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let target_user_id = i64_arg(args, "userId")?;
    let session_id = string_arg(args, "sessionId")?;
    auth_records_actions::revoke_session_for_user(
        state.runtime(),
        user_id,
        target_user_id,
        &session_id,
    )
    .await?;
    Ok(Value::Null)
}
