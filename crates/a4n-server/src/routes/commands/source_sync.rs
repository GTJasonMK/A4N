use serde_json::Value;

use super::{ApiError, ServerState, command_input, i64_arg, source_sync_actions, to_value};

pub(in crate::routes::commands) async fn preview_source_sync(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let derived_project_id = i64_arg(args, "derivedProjectId")?;
    to_value(
        source_sync_actions::preview_source_sync(state.runtime(), user_id, derived_project_id)
            .await?,
    )
}

pub(in crate::routes::commands) async fn apply_source_sync_decisions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: source_sync_actions::ApplySourceSyncInput =
        command_input(args, "apply_source_sync_decisions")?;
    to_value(
        source_sync_actions::apply_source_sync_decisions(state.runtime(), user_id, input).await?,
    )
}

pub(in crate::routes::commands) async fn detach_derived_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let derived_project_id = i64_arg(args, "derivedProjectId")?;
    to_value(
        source_sync_actions::detach_derived_project(state.runtime(), user_id, derived_project_id)
            .await?,
    )
}
