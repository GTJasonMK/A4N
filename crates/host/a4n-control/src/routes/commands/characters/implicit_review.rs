use serde_json::Value;

use super::super::{
    ApiError, ServerState, character_implicit_review_actions, strict_command_input,
    strict_runtime_command_input, to_value,
};
use super::input::character_web_input_command;

character_web_input_command!(
    get_implicit_review,
    character_implicit_review_actions::GetImplicitReviewInput,
    character_implicit_review_actions::get_implicit_review,
    strict_command_input,
);

character_web_input_command!(
    check_implicit_update,
    character_implicit_review_actions::CheckImplicitUpdateInput,
    character_implicit_review_actions::check_implicit_update,
    strict_runtime_command_input,
);

pub(in crate::routes::commands) async fn preview_check_implicit_update_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: character_implicit_review_actions::CheckImplicitUpdateInput =
        strict_runtime_command_input(args, "preview_check_implicit_update_request")?;
    let preview = character_implicit_review_actions::preview_check_implicit_update_request(
        state.runtime(),
        user_id,
        &input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

character_web_input_command!(
    apply_implicit_update_decision,
    character_implicit_review_actions::ApplyImplicitUpdateDecisionInput,
    character_implicit_review_actions::apply_implicit_update_decision,
    strict_command_input,
);
