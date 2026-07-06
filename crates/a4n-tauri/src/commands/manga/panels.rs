use a4n_runtime::AppState;
use a4n_runtime::actions::dependency_conflict::DownstreamConflictStrategy;
use a4n_runtime::actions::manga_records::{
    self as manga_records_actions, BindReferenceImageInput, CreateMangaPromptInput,
    MangaPromptSummary, OptimizeMangaPromptInput, UpdateMangaPromptInput,
};
use tauri::State;

#[tauri::command]
pub async fn list_manga_panels(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<MangaPromptSummary>, String> {
    manga_records_actions::list_manga_panels(state.inner(), user_id, project_id, chapter_id).await
}

#[tauri::command]
pub async fn create_manga_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: CreateMangaPromptInput,
) -> Result<MangaPromptSummary, String> {
    manga_records_actions::create_manga_prompt(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        input,
    )
    .await
}

#[tauri::command]
pub async fn bind_manga_reference_image(
    state: State<'_, AppState>,
    user_id: i64,
    input: BindReferenceImageInput,
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
    project_id: i64,
    chapter_id: i64,
) -> Result<(), String> {
    manga_records_actions::delete_manga_by_chapter(state.inner(), user_id, project_id, chapter_id)
        .await
}

#[tauri::command]
pub async fn delete_manga_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    prompt_id: i64,
    downstream_strategy: Option<String>,
) -> Result<(), String> {
    let downstream_strategy =
        DownstreamConflictStrategy::from_optional_str(downstream_strategy.as_deref())?;
    manga_records_actions::delete_manga_prompt(
        state.inner(),
        user_id,
        prompt_id,
        downstream_strategy,
    )
    .await
}
