use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, i64_arg, image_generation_actions, nullable_i64_arg, required_arg,
    required_json_i32, string_arg, to_value,
};

pub(in crate::routes::commands) async fn preview_scene_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let prompt = string_arg(args, "prompt")?;
    let config_id = nullable_i64_arg(args, "configId")?;
    let chapter_id = nullable_i64_arg(args, "chapterId")?;

    let preview = image_generation_actions::preview_scene_image_request(
        state.runtime(),
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_page_image_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "preview_page_image_request")?;
    let page_number = required_json_i32(args, "pageNumber", "preview_page_image_request")?;

    let preview = image_generation_actions::preview_page_image_request(
        state.runtime(),
        user_id,
        project_id,
        chapter_number,
        page_number,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

pub(in crate::routes::commands) async fn preview_batch_images_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let requests_json = required_arg(args, "requestsJson")?.clone();

    let preview = image_generation_actions::preview_batch_images_request(
        state.runtime(),
        user_id,
        project_id,
        requests_json,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}
