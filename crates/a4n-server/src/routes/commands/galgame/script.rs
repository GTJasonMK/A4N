use a4n_runtime::host_api::galgame_records as galgame_records_actions;
use serde_json::Value;

use crate::state::ServerState;

use super::super::{ApiError, command_input, i64_arg, to_value};

pub(in crate::routes::commands) async fn get_galgame_script(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::get_galgame_script(
            state.runtime(),
            user_id,
            command_input(args, "get_galgame_script")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn update_galgame_line(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::update_galgame_line(
            state.runtime(),
            user_id,
            project_id,
            command_input(args, "update_galgame_line")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn insert_galgame_line(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::insert_galgame_line(
            state.runtime(),
            user_id,
            project_id,
            command_input(args, "insert_galgame_line")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn update_galgame_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::update_galgame_scene(
            state.runtime(),
            user_id,
            project_id,
            command_input(args, "update_galgame_scene")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn delete_galgame_line(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    galgame_records_actions::delete_galgame_line(
        state.runtime(),
        user_id,
        project_id,
        command_input(args, "delete_galgame_line")?,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn reorder_galgame_lines(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::reorder_galgame_lines(
            state.runtime(),
            user_id,
            project_id,
            command_input(args, "reorder_galgame_lines")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn update_galgame_choices(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        galgame_records_actions::update_galgame_choices(
            state.runtime(),
            user_id,
            project_id,
            command_input(args, "update_galgame_choices")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}
