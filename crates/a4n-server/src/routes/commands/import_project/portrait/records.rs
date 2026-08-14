use crate::state::ServerState;
use a4n_api_contract::portrait::{
    GetPrimaryPortraitInput, ListPortraitsByProjectInput, SetPrimaryPortraitInput,
};
use serde_json::Value;

use super::super::super::{
    ApiError, portrait_generation_actions, portrait_records_actions, strict_command_input, to_value,
};

pub(in crate::routes::commands) async fn list_portraits_by_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: ListPortraitsByProjectInput =
        strict_command_input(args, "list_portraits_by_project")?;
    let portraits =
        portrait_records_actions::list_portraits_by_project(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(portraits)
}

pub(in crate::routes::commands) async fn get_primary_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GetPrimaryPortraitInput = strict_command_input(args, "get_primary_portrait")?;
    let portrait = portrait_records_actions::get_primary_portrait(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(portrait)
}

pub(in crate::routes::commands) async fn set_primary_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: SetPrimaryPortraitInput = strict_command_input(args, "set_primary_portrait")?;
    portrait_records_actions::set_primary_portrait(state.runtime(), user_id, input)
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
        strict_command_input(args, "upload_portrait")?;
    let portrait = portrait_generation_actions::upload_portrait(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(portrait)
}
