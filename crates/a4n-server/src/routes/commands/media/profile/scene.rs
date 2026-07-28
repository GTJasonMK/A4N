use serde_json::{Value, json};

use super::super::super::{ApiError, ServerState, command_input, image_generation_actions};

pub(in crate::routes::commands) async fn generate_scene_profile_image_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::SceneProfileImageInput =
        command_input(args, "generate_scene_profile_image_prompt")?;
    let output = image_generation_actions::generate_scene_profile_image_prompt(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(json!(output))
}
