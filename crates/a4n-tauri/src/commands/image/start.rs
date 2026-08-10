use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
    image_generation::StartImageGenerationInput,
};
use a4n_runtime::AppState;
use tauri::State;

#[tauri::command]
pub async fn start_image_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartImageGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    a4n_runtime::host_api::image_generation::start_image_generation(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}
