use crate::state::ServerState;
use serde_json::Value;

use super::super::super::{
    ApiError, command_input, i64_arg, portrait_generation_actions, portrait_records_actions,
    to_value,
};

pub(in crate::routes::commands) async fn list_portraits_by_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let portraits =
        portrait_records_actions::list_portraits_by_project(state.runtime(), user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(portraits)
}

pub(in crate::routes::commands) async fn get_primary_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let profile_id = i64_arg(args, "profileId")?;
    let portrait =
        portrait_records_actions::get_primary_portrait(state.runtime(), user_id, profile_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(portrait)
}

pub(in crate::routes::commands) async fn set_primary_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let portrait_id = i64_arg(args, "portraitId")?;
    portrait_records_actions::set_primary_portrait(state.runtime(), user_id, portrait_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn upload_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: portrait_generation_actions::UploadPortraitInput =
        command_input(args, "upload_portrait")?;
    let portrait = portrait_generation_actions::upload_portrait(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(portrait)
}
