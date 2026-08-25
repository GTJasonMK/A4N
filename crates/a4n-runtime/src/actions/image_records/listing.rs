use crate::{
    AppState,
    actions::{
        app_error_message,
        image_generation::{GeneratedImageSummary, to_summary},
    },
};

use super::{
    ListImagesByChapterInput, ListImagesByProjectInput, ListImagesByProjectsInput,
    ListVideoReferenceImagesInput,
};

pub async fn list_images_by_project(
    state: &AppState,
    user_id: i64,
    input: ListImagesByProjectInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let rows = state
        .orchestrator()
        .list_generated_images_for_user_validated(user_id, input.project_id)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_images_by_projects(
    state: &AppState,
    user_id: i64,
    input: ListImagesByProjectsInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let rows = state
        .orchestrator()
        .list_generated_images_for_projects_for_user_validated(user_id, input.project_ids)
        .await
        .map_err(app_error_message)?;
    Ok(rows.iter().map(to_summary).collect())
}

pub async fn list_images_by_chapter(
    state: &AppState,
    user_id: i64,
    input: ListImagesByChapterInput,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let rows = state
        .orchestrator()
        .list_chapter_images_for_user_validated(user_id, input.project_id, input.chapter_id)
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
        .orchestrator()
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
