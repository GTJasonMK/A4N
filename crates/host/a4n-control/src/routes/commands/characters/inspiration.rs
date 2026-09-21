use serde_json::Value;

use super::super::{
    ApiError, ServerState, inspiration_records_actions, strict_command_input,
    strict_runtime_command_input, to_value,
};

pub(in crate::routes::commands) async fn get_inspiration_history(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::InspirationHistoryInput =
        strict_command_input(args, "get_inspiration_history")?;
    to_value(
        inspiration_records_actions::get_inspiration_history(state.runtime(), user_id, input)
            .await?,
    )
}

pub(in crate::routes::commands) async fn preview_inspiration_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: inspiration_records_actions::PreviewInspirationRequestInput =
        strict_runtime_command_input(args, "preview_inspiration_request")?;
    to_value(
        inspiration_records_actions::preview_inspiration_request(state.runtime(), user_id, input)
            .await?,
    )
}
