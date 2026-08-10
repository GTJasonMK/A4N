use a4n_runtime::AppState;
use a4n_runtime::host_api::manga_records::{
    self as manga_records_actions, BindMangaReferenceImageInput, DeleteMangaPromptInput,
    MangaChapterInput, MangaPromptSummary, OptimizeMangaPromptInput, UpdateMangaPromptInput,
};
use tauri::State;

#[tauri::command]
pub async fn list_manga_panels(
    state: State<'_, AppState>,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<Vec<MangaPromptSummary>, String> {
    manga_records_actions::list_manga_panels(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn bind_manga_reference_image(
    state: State<'_, AppState>,
    user_id: i64,
    input: BindMangaReferenceImageInput,
) -> Result<MangaPromptSummary, String> {
    manga_records_actions::bind_manga_reference_image(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn update_manga_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    manga_records_actions::update_manga_prompt(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn optimize_manga_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: OptimizeMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    manga_records_actions::optimize_manga_prompt(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn delete_manga_by_chapter(
    state: State<'_, AppState>,
    user_id: i64,
    input: MangaChapterInput,
) -> Result<(), String> {
    manga_records_actions::delete_manga_by_chapter(state.inner(), user_id, input).await
}

#[tauri::command]
pub async fn delete_manga_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: DeleteMangaPromptInput,
) -> Result<(), String> {
    manga_records_actions::delete_manga_prompt(state.inner(), user_id, input).await
}
