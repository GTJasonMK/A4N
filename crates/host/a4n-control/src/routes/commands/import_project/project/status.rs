use super::macros::{project_web_project_value_command, project_web_status_command};
use crate::routes::commands::{ApiError, project_status_actions, required_arg, to_value};
use crate::state::ServerState;

project_web_status_command!(
    forward_project_status,
    project_status_actions::forward_project_status
);
project_web_project_value_command!(
    get_project_status_machine,
    project_status_actions::get_project_status_machine
);

pub(in crate::routes::commands) async fn preview_rollback(
    state: &ServerState,
    user_id: i64,
    args: &serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    let project_id = crate::routes::commands::i64_arg(args, "projectId")?;
    let expected_from = project_status_arg(args, "expectedFrom", "preview_rollback")?;
    let target = project_status_arg(args, "target", "preview_rollback")?;
    to_value(
        project_status_actions::preview_rollback(
            state.runtime(),
            user_id,
            project_id,
            expected_from,
            target,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn rollback_project_status(
    state: &ServerState,
    user_id: i64,
    args: &serde_json::Value,
) -> Result<serde_json::Value, ApiError> {
    let project_id = crate::routes::commands::i64_arg(args, "projectId")?;
    let expected_from = project_status_arg(args, "expectedFrom", "rollback_project_status")?;
    let target = project_status_arg(args, "target", "rollback_project_status")?;
    to_value(
        project_status_actions::rollback_project_status(
            state.runtime(),
            user_id,
            project_id,
            expected_from,
            target,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

fn project_status_arg(
    args: &serde_json::Value,
    field: &str,
    command: &str,
) -> Result<a4n_api_contract::project::ProjectStatus, ApiError> {
    serde_json::from_value(required_arg(args, field)?.clone())
        .map_err(|error| ApiError::bad_request(format!("{command}.{field} 无效: {error}")))
}
