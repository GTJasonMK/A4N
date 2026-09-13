use a4n_api_contract::image_generation::{
    GenerateBatchImagesInput, GeneratePageImageInput, GenerateSceneImageInput,
};
use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, image_generation_actions, strict_runtime_command_input, to_value,
};

pub(in crate::routes::commands) async fn preview_scene_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GenerateSceneImageInput =
        strict_runtime_command_input(args, "preview_scene_image_request")?;
    let preview =
        image_generation_actions::preview_scene_image_request(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_page_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GeneratePageImageInput =
        strict_runtime_command_input(args, "preview_page_image_request")?;
    let preview =
        image_generation_actions::preview_page_image_request(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_batch_images_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: GenerateBatchImagesInput =
        strict_runtime_command_input(args, "preview_batch_images_request")?;
    let preview =
        image_generation_actions::preview_batch_images_request(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}
