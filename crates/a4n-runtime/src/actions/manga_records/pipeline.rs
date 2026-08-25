use crate::{AppState, actions::app_error_message};
use a4n_api_contract::manga::{
    MangaChapterInput, MangaPipelineStatusSummary, MangaPipelineStatusesInput,
};

pub async fn get_manga_pipeline_status(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<MangaPipelineStatusSummary, String> {
    state
        .orchestrator()
        .get_manga_pipeline_status_validated(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn list_manga_pipeline_statuses(
    state: &AppState,
    user_id: i64,
    input: MangaPipelineStatusesInput,
) -> Result<Vec<MangaPipelineStatusSummary>, String> {
    state
        .orchestrator()
        .list_manga_pipeline_statuses_validated(user_id, input.project_id, &input.chapter_ids)
        .await
        .map_err(app_error_message)
}

pub async fn cancel_manga_pipeline(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<i64, String> {
    state
        .orchestrator()
        .cancel_manga_pipeline_validated(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}
