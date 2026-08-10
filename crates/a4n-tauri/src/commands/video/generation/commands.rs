use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
    video::StartVideoGenerationInput,
};
use a4n_runtime::AppState;
use tauri::State;

#[tauri::command]
pub async fn start_video_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartVideoGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    a4n_runtime::host_api::video_generation::start_video_generation(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}
