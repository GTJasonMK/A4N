use a4n_runtime::AppState;
use a4n_runtime::host_api::video_generation::{
    self as video_generation_actions, GeneratedVideoSummary,
};
use tauri::State;

#[tauri::command]
pub async fn poll_video_job(
    state: State<'_, AppState>,
    user_id: i64,
    video_id: i64,
) -> Result<GeneratedVideoSummary, String> {
    video_generation_actions::poll_video_job(state.inner(), user_id, video_id).await
}
