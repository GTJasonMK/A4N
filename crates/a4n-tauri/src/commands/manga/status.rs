use a4n_runtime::AppState;
use a4n_runtime::host_api::manga_records::{self as manga_records_actions, PipelineStatusSummary};
use tauri::State;

#[tauri::command]
pub async fn get_manga_pipeline_status(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<PipelineStatusSummary, String> {
    manga_records_actions::get_manga_pipeline_status(state.inner(), user_id, project_id, chapter_id)
        .await
}

#[tauri::command]
pub async fn cancel_manga_pipeline(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<i64, String> {
    manga_records_actions::cancel_manga_pipeline(state.inner(), user_id, project_id, chapter_id)
        .await
}
