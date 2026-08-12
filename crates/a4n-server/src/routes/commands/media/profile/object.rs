use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, image_generation_actions, strict_command_input,
    strict_runtime_command_input, to_value,
};

pub(in crate::routes::commands) async fn generate_object_profile_image_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::ObjectProfileImageInput =
        strict_command_input(args, "generate_object_profile_image_prompt")?;
    let output = image_generation_actions::generate_object_profile_image_prompt(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(output)
}

pub(in crate::routes::commands) async fn preview_object_profile_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::ObjectProfileImageInput =
        strict_runtime_command_input(args, "preview_object_profile_image_request")?;
    let preview = image_generation_actions::preview_object_profile_image_request(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}
