use a4n_api_contract::{
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
    outline::StartOutlineGenerationInput,
};
use a4n_runtime::{AppState, host_api::outline_records};
use tauri::State;

#[tauri::command]
pub async fn start_outline_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartOutlineGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    outline_records::start_outline_generation_typed(state.inner(), user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}
