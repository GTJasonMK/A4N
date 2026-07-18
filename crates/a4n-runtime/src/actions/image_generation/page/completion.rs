use chrono::NaiveDateTime;

use crate::{AppState, actions::app_error_message};

pub(super) async fn complete_manga_page_image(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    page_prompt_id: i64,
    image_id: i64,
    updated_at: NaiveDateTime,
) -> Result<(), String> {
    state
        .orchestrator
        .mark_manga_page_prompt_image_completed_with_image(page_prompt_id, image_id, updated_at)
        .await
        .map_err(app_error_message)?;

    let old_paths = state
        .orchestrator
        .delete_manga_page_image_records(project_id, chapter_id, page_number, Some(image_id))
        .await
        .map_err(|error| error.to_string())?;
    for path in old_paths {
        crate::file_utils::remove_data_file_in_data_dir(
            &state.config.data_dir,
            &path,
            "旧漫画页图片",
        )?;
    }

    state
        .orchestrator
        .sync_project_asset_lineage(project_id)
        .await
        .map_err(app_error_message)
        .map(|_| ())
}
