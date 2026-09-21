use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, direct_command_input, image_records_actions, strict_command_input,
    to_value,
};

pub(in crate::routes::commands) async fn list_images_by_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::ListImagesByProjectInput =
        strict_command_input(args, "list_images_by_project")?;
    let images = image_records_actions::list_images_by_project(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(in crate::routes::commands) async fn list_images_by_projects(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::ListImagesByProjectsInput =
        strict_command_input(args, "list_images_by_projects")?;
    let images = image_records_actions::list_images_by_projects(state.runtime(), user_id, input)
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
        strict_command_input(args, "list_images_by_chapter")?;
    let images = image_records_actions::list_images_by_chapter(state.runtime(), user_id, input)
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
        strict_command_input(args, "list_video_reference_images")?;
    let images =
        image_records_actions::list_video_reference_images(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(images)
}

pub(in crate::routes::commands) async fn read_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: a4n_api_contract::image_read::ReadImageInput =
        direct_command_input(args, "read_image")?;
    to_value(
        image_records_actions::read_image(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::from)?,
    )
}

pub(in crate::routes::commands) async fn update_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::UpdateGeneratedImageInput =
        strict_command_input(args, "update_image")?;
    let updated = image_records_actions::update_image(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn upload_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::UploadImageInput =
        strict_command_input(args, "upload_image")?;
    let image = image_records_actions::upload_image(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(image)
}

pub(in crate::routes::commands) async fn delete_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: image_records_actions::DeleteImageInput =
        strict_command_input(args, "delete_image")?;
    image_records_actions::delete_image(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
