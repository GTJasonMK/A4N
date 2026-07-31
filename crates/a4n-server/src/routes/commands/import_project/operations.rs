use super::super::{
    ApiError, command_input, generation_analytics_actions, i64_arg, logging_actions,
    require_admin_caller, storage_paths_actions, to_value,
};
use crate::state::ServerState;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn get_storage_paths(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    logging_actions::ensure_log_diagnostics_admin(state.runtime(), user_id).await?;
    to_value(storage_paths_actions::get_storage_paths(state.runtime()))
}

pub(in crate::routes::commands) async fn list_orphan_project_data_dirs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    require_admin_caller(state, user_id, args).await?;
    let caller_user_id = i64_arg(args, "callerUserId")?;
    let result =
        storage_paths_actions::list_orphan_project_data_dirs(state.runtime(), caller_user_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn cleanup_orphan_project_data_dirs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: storage_paths_actions::CleanupOrphanProjectDataDirsInput =
        command_input(args, "cleanup_orphan_project_data_dirs")?;
    if input.caller_user_id != user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    let result = storage_paths_actions::cleanup_orphan_project_data_dirs(state.runtime(), input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(in crate::routes::commands) async fn get_log_dir(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    let log_dir = storage_paths_actions::log_dir_path(state.runtime());
    let path = logging_actions::log_dir_path_for_admin(state.runtime(), user_id, &log_dir).await?;
    Ok(json!(path))
}

pub(in crate::routes::commands) async fn read_recent_logs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
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
