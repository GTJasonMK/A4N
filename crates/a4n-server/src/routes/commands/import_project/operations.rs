use super::super::{
    ApiError, command_input, empty_command_args, generation_analytics_actions, logging_actions,
    reject_unknown_command_args, storage_paths_actions, to_value,
};
use crate::state::ServerState;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn get_storage_paths(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "get_storage_paths")?;
    to_value(storage_paths_actions::get_storage_paths(state.runtime(), user_id).await?)
}

pub(in crate::routes::commands) async fn list_orphan_project_data_dirs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "list_orphan_project_data_dirs")?;
    let result = storage_paths_actions::list_orphan_project_data_dirs(state.runtime(), user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn cleanup_orphan_project_data_dirs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "cleanup_orphan_project_data_dirs")?;
    let result = storage_paths_actions::cleanup_orphan_project_data_dirs(state.runtime(), user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn get_log_dir(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    empty_command_args(args, "get_log_dir")?;
    let log_dir = storage_paths_actions::log_dir_path(state.runtime());
    let path = logging_actions::log_dir_path_for_admin(state.runtime(), user_id, &log_dir).await?;
    Ok(json!(path))
}

pub(in crate::routes::commands) async fn read_recent_logs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    reject_unknown_command_args(args, "read_recent_logs", &["input"])?;
    let input: logging_actions::ReadLogsInput = command_input(args, "read_recent_logs")?;
    let log_dir = storage_paths_actions::log_dir_path(state.runtime());
    let result =
        logging_actions::read_recent_logs_for_admin(state.runtime(), user_id, &log_dir, input)
            .await?;
    to_value(result)
}

pub(in crate::routes::commands) async fn set_log_level(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    reject_unknown_command_args(args, "set_log_level", &["input"])?;
    let input: a4n_runtime::host_api::logging::SetLogLevelInput =
        command_input(args, "set_log_level")?;
    logging_actions::ensure_log_diagnostics_admin(state.runtime(), user_id).await?;
    crate::logging::set_log_level(input).map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn get_generation_call_analytics(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    reject_unknown_command_args(args, "get_generation_call_analytics", &["input"])?;
    let input: generation_analytics_actions::GenerationCallAnalyticsInput =
        command_input(args, "get_generation_call_analytics")?;
    let result = generation_analytics_actions::get_generation_call_analytics_for_admin(
        state.runtime(),
        user_id,
        input,
    )
    .await?;
    to_value(result)
}
