use crate::{AppState, actions::app_error_message};
use a4n_api_contract::image::GeneratedImageSummary;

use super::{
    ListImagesByChapterInput, ListImagesByProjectInput, ListImagesByProjectsInput,
    ListVideoReferenceImagesInput,
};

pub async fn list_images_by_project(
    state: &AppState,
    user_id: i64,
    input: ListImagesByProjectInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    state
        .image_library_read_application()
        .list_by_project(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn list_images_by_projects(
    state: &AppState,
    user_id: i64,
    input: ListImagesByProjectsInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    state
        .image_library_read_application()
        .list_by_projects(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn list_images_by_chapter(
    state: &AppState,
    user_id: i64,
    input: ListImagesByChapterInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    state
        .image_library_read_application()
        .list_by_chapter(user_id, input)
        .await
        .map_err(app_error_message)
}

pub async fn list_video_reference_images(
    state: &AppState,
    user_id: i64,
    input: ListVideoReferenceImagesInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    state
        .image_record_read_application()
        .list_video_reference_images(user_id, input)
        .await
        .map_err(app_error_message)
}
