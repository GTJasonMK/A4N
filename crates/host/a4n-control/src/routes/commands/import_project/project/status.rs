use super::macros::{project_web_project_value_command, project_web_status_command};
use crate::routes::commands::{ApiError, project_status_actions, required_arg, to_value};
use crate::state::ServerState;

project_web_status_command!(
    forward_project_status,
    project_status_actions::forward_project_status
);
project_web_status_command!(
    rollback_project_status,
    project_status_actions::rollback_project_status
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
    let target = serde_json::from_value(required_arg(args, "target")?.clone())
        .map_err(|error| ApiError::bad_request(format!("preview_rollback.target 无效: {error}")))?;
    to_value(
        project_status_actions::preview_rollback(state.runtime(), user_id, project_id, target)
            .await
            .map_err(ApiError::bad_request)?,
    )
}
