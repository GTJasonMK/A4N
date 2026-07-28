use serde_json::Value;

use super::super::{
    ApiError, ServerState, command_input, i64_arg, inspiration_records_actions, to_value, u64_arg,
};

pub(in crate::routes::commands) async fn get_inspiration_history(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let limit = u64_arg(args, "limit")?;
    to_value(
        inspiration_records_actions::get_inspiration_history(
            state.runtime(),
            user_id,
            project_id,
            limit,
        )
        .await?,
    )
}

pub(in crate::routes::commands) async fn preview_inspiration_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::PreviewInspirationRequestInput =
        command_input(args, "preview_inspiration_request")?;
    to_value(
        inspiration_records_actions::preview_inspiration_request(state.runtime(), user_id, input)
            .await?,
    )
}
