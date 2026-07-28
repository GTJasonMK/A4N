use crate::{AppState, actions::app_error_message};

use super::PipelineStatusSummary;

pub async fn get_manga_pipeline_status(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<PipelineStatusSummary, String> {
    state
        .manga()
        .get_manga_pipeline_status_validated(user_id, project_id, chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn cancel_manga_pipeline(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<i64, String> {
    state
        .manga()
        .cancel_manga_pipeline_validated(user_id, project_id, chapter_id)
        .await
        .map_err(app_error_message)
}
