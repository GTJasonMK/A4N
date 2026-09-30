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
        .manga_pipeline_read_application()
        .status(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)
}

pub async fn list_manga_pipeline_statuses(
    state: &AppState,
    user_id: i64,
    input: MangaPipelineStatusesInput,
) -> Result<Vec<MangaPipelineStatusSummary>, String> {
    state
        .manga_pipeline_read_application()
        .statuses(user_id, input.project_id, input.chapter_ids)
        .await
        .map_err(app_error_message)
}

pub async fn cancel_manga_pipeline(
    state: &AppState,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<i64, String> {
    let status = state
        .manga_pipeline_read_application()
        .status(user_id, input.project_id, input.chapter_id)
        .await
        .map_err(app_error_message)?;
    let handle = status
        .task_handle
        .filter(|_| status.can_cancel)
        .ok_or_else(|| "没有可取消的漫画流水线 Generation Task".to_string())?;
    let cancellable_pages = status
        .pending
        .checked_add(status.generating)
        .ok_or_else(|| "可取消的漫画页面数量溢出".to_string())?;
    state
        .generation_task_mutation_application()
        .request_generation_cancel(user_id, &handle.task_id, handle.attempt)
        .await
        .map_err(|error| error.to_string())?;
    Ok(cancellable_pages)
}
