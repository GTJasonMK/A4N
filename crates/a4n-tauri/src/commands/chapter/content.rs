use a4n_runtime::AppState;
use a4n_runtime::host_api::chapter_content as chapter_content_actions;
use chapter_content_actions::{ChapterSummary, ImportChapterInput, UpdateChapterContentInput};
use tauri::State;

#[tauri::command]
pub async fn list_chapters(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterSummary>, String> {
    chapter_content_actions::list_chapters(state.inner(), user_id, project_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn import_chapter(
    state: State<'_, AppState>,
    user_id: i64,
    input: ImportChapterInput,
) -> Result<serde_json::Value, String> {
    let imported = chapter_content_actions::import_chapter(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!(imported))
}

#[tauri::command]
pub async fn update_chapter_content(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateChapterContentInput,
) -> Result<ChapterSummary, String> {
    chapter_content_actions::update_chapter_content(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}
