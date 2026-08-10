use a4n_runtime::AppState;
use a4n_runtime::host_api::outline_records as outline_actions;
use outline_actions::{
    ChapterOutlineSummary, GenerationProgressSummary, OutlineMutationResult, PartOutlineSummary,
    SaveChapterOutlineInput, SaveChapterOutlineResult, SavePartOutlineInput, SavePartOutlineResult,
    UpdateChapterOutlineInput, UpdateChapterOutlineResult,
};
use tauri::State;

#[tauri::command]
pub async fn list_parts(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<PartOutlineSummary>, String> {
    outline_actions::list_parts(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_part_outline(
    state: State<'_, AppState>,
    user_id: i64,
    input: SavePartOutlineInput,
) -> Result<SavePartOutlineResult, String> {
    outline_actions::save_part_outline(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn sync_outline_materials(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<OutlineMutationResult, String> {
    outline_actions::sync_outline_materials(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_latest_parts(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<OutlineMutationResult, String> {
    outline_actions::delete_latest_parts(state.inner(), user_id, project_id, count)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_generation_progress(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<GenerationProgressSummary, String> {
    outline_actions::get_generation_progress(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_chapter_outlines(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ChapterOutlineSummary>, String> {
    outline_actions::list_chapter_outlines(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn save_chapter_outline(
    state: State<'_, AppState>,
    user_id: i64,
    input: SaveChapterOutlineInput,
) -> Result<SaveChapterOutlineResult, String> {
    outline_actions::save_chapter_outline(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn delete_latest_chapter_outlines(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    count: u64,
) -> Result<OutlineMutationResult, String> {
    outline_actions::delete_latest_chapter_outlines(state.inner(), user_id, project_id, count)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_chapter_outline(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateChapterOutlineInput,
) -> Result<UpdateChapterOutlineResult, String> {
    outline_actions::update_chapter_outline(state.inner(), user_id, input)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_chapter_outlines_pacing(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    outline_actions::get_chapter_outlines_pacing(state.inner(), user_id, project_id)
        .await
        .map_err(|e| e.to_string())
}
