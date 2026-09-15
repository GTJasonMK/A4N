use a4n_runtime::host_api::galgame_records as galgame_records_actions;
use serde_json::Value;

use crate::state::ServerState;

use super::super::{ApiError, command_input, to_value};

pub(in crate::routes::commands) async fn plan_galgame_assets(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::plan_galgame_assets(
            state.runtime(),
            user_id,
            command_input(args, "plan_galgame_assets")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn run_galgame_asset_generation(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::run_galgame_asset_generation(
            state.runtime(),
            user_id,
            command_input(args, "run_galgame_asset_generation")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn bind_galgame_media(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::bind_galgame_media(
            state.runtime(),
            user_id,
            command_input(args, "bind_galgame_media")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn unbind_galgame_media(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::unbind_galgame_media(
            state.runtime(),
            user_id,
            command_input(args, "unbind_galgame_media")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn import_galgame_audio(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::import_galgame_audio(
            state.runtime(),
            user_id,
            command_input(args, "import_galgame_audio")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn export_galgame_webgal(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::export_galgame_webgal(
            state.runtime(),
            user_id,
            command_input(args, "export_galgame_webgal")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}
