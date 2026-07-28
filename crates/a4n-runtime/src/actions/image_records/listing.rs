use crate::{
    AppState,
    actions::{
        app_error_message,
        image_generation::{GeneratedImageSummary, to_summary},
    },
};

use super::ListVideoReferenceImagesInput;

pub async fn list_images_by_project(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let rows = state
        .media()
        .list_generated_images_for_user_validated(user_id, project_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_images_by_chapter(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let rows = state
        .media()
        .list_chapter_images_for_user_validated(user_id, project_id, chapter_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_video_reference_images(
    state: &AppState,
    user_id: i64,
    input: ListVideoReferenceImagesInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let rows = state
        .media()
        .list_video_reference_images_for_user_validated(
            user_id,
            input.project_id,
            input.chapter_id,
            input.image_project_ids,
        )
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}
