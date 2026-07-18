use super::macros::{project_summary_value, project_web_input_summary_command};
use crate::routes::commands::{ApiError, i64_arg, string_arg};
use crate::state::ServerState;
use serde_json::Value;

project_web_input_summary_command!(
    duplicate_project,
    a4n_runtime::host_api::project_duplication::DuplicateProjectInput,
    a4n_runtime::host_api::project_duplication::duplicate_project
);

pub(in crate::routes::commands) async fn branch_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let branch_title = string_arg(args, "branchTitle")?;
    let at_status = string_arg(args, "atStatus")?;
    let project = a4n_runtime::host_api::project_duplication::branch_project(
        state.runtime(),
        user_id,
        project_id,
        branch_title,
        at_status,
    )
    .await
    .map_err(ApiError::bad_request)?;
    project_summary_value(state, user_id, &project).await
}
