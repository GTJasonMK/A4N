use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, f32_arg, i64_arg, image_records_actions, to_value,
};

pub(in crate::routes::commands) async fn list_images_by_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let images =
        image_records_actions::list_images_by_project(state.runtime(), user_id, project_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(in crate::routes::commands) async fn list_images_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::ListImagesByChapterInput =
        command_input(args, "list_images_by_chapter")?;
    let images = image_records_actions::list_images_by_chapter(
        state.runtime(),
        user_id,
        input.project_id,
        input.chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(in crate::routes::commands) async fn list_video_reference_images(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::ListVideoReferenceImagesInput =
        command_input(args, "list_video_reference_images")?;
    let images =
        image_records_actions::list_video_reference_images(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(in crate::routes::commands) async fn toggle_image_favorite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    let updated = image_records_actions::toggle_image_favorite(state.runtime(), user_id, image_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn update_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    let input: image_records_actions::UpdateGeneratedImageInput =
        command_input(args, "update_image")?;
    let updated = image_records_actions::update_image(state.runtime(), user_id, image_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn upload_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: image_records_actions::UploadImageInput = command_input(args, "upload_image")?;
    let image = image_records_actions::upload_image(state.runtime(), user_id, project_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(image)
}

pub(in crate::routes::commands) async fn delete_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    image_records_actions::delete_image(state.runtime(), user_id, image_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn score_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let image_id = i64_arg(args, "imageId")?;
    let score = f32_arg(args, "score")?;
    let updated = image_records_actions::score_image(state.runtime(), user_id, image_id, score)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}
