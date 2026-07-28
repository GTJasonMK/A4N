use a4n_api_contract::{
    chapter::StartChapterGenerationInput,
    generation_tasks::{GenerationTaskHandle, HostErrorResponse},
};
use a4n_runtime::{AppState, host_api::chapter_generation};
use tauri::State;

#[tauri::command]
pub async fn start_chapter_generation(
    state: State<'_, AppState>,
    user_id: i64,
    input: StartChapterGenerationInput,
) -> Result<GenerationTaskHandle, HostErrorResponse> {
    chapter_generation::start_chapter_generation_typed(&state, user_id, input)
        .await
        .map_err(crate::commands::generation_tasks::map_access_error)
}
