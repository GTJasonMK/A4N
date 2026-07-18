use a4n_core::event::channel::TaskType;
use serde_json::{Value, json};

use super::super::super::{
    ApiError, ServerState, command_input, finish_web_task, image_generation_actions,
    optional_string_field, register_web_task, sanitize_optional_task_id, string_action_result,
    to_value, web_fire_and_forget_progress_reporter,
};
use super::super::image_task::web_image_generation_command;

pub(in crate::routes::commands) async fn generate_object_profile_image_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::ObjectProfileImageInput =
        command_input(args, "generate_object_profile_image_prompt")?;
    let output = image_generation_actions::generate_object_profile_image_prompt(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(json!(output))
}

web_image_generation_command!(
    generate_object_profile_image,
    "准备生成物体素材图",
    "物体素材图生成完成",
    |state, user_id, args, progress, scope| {
        let input: image_generation_actions::ObjectProfileImageInput =
            command_input(args, "generate_object_profile_image")?;
        let scope = image_generation_actions::object_profile_image_generation_task_scope(
            state.runtime(),
            user_id,
            &input,
        )
        .await
        .map_err(ApiError::bad_request)?;
    },
    image_generation_actions::generate_object_profile_image(
        state.runtime(),
        user_id,
        input,
        Some(&progress),
    )
    .await,
);

pub(in crate::routes::commands) async fn preview_object_profile_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_generation_actions::ObjectProfileImageInput =
        command_input(args, "preview_object_profile_image_request")?;
    let preview = image_generation_actions::preview_object_profile_image_request(
        state.runtime(),
        user_id,
        input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}
