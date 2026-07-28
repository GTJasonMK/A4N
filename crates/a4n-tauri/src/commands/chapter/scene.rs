use crate::commands::request_preview::ModelRequestPreview;
use crate::progress_task::WorkflowProgressTask;
use a4n_runtime::AppState;
use a4n_runtime::host_api::chapter_scene::{
    self as chapter_scene_actions, ChapterSceneSummary, GenerateChapterSceneInput,
    ReorderScenesInput, UpdateChapterSceneInput,
};
use tauri::State;

#[tauri::command]
pub async fn list_chapter_scenes(
    state: State<'_, AppState>,
    user_id: i64,
    chapter_id: i64,
) -> Result<Vec<ChapterSceneSummary>, String> {
    chapter_scene_actions::list_chapter_scenes(&state, user_id, chapter_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn update_chapter_scene(
    state: State<'_, AppState>,
    user_id: i64,
    input: UpdateChapterSceneInput,
) -> Result<ChapterSceneSummary, String> {
    chapter_scene_actions::update_chapter_scene(&state, user_id, input)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn delete_chapter_scene(
    state: State<'_, AppState>,
    user_id: i64,
    scene_id: i64,
) -> Result<(), String> {
    chapter_scene_actions::delete_chapter_scene(&state, user_id, scene_id)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn reorder_chapter_scenes(
    state: State<'_, AppState>,
    user_id: i64,
    input: ReorderScenesInput,
) -> Result<Vec<ChapterSceneSummary>, String> {
    chapter_scene_actions::reorder_chapter_scenes(&state, user_id, input)
        .await
        .map_err(|err| err.to_string())
}

tauri_chapter_progress_command!(
    generate_chapter_scene(input: GenerateChapterSceneInput) -> ChapterSceneSummary,
    {
        let scope =
            chapter_scene_actions::generate_chapter_scene_task_scope(state.inner(), user_id, &input)
                .await
                .map_err(|err| err.to_string())?;
    },
    project_id: scope.project_id,
    total: scope.total,
    "正在准备场景生成",
    |_window, state, user_id, progress| generate_chapter_scene_with_progress(
        &state,
        user_id,
        input,
        Some(progress),
    ),
    |_| {},
);

async fn generate_chapter_scene_with_progress(
    state: &State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterSceneInput,
    progress: Option<&WorkflowProgressTask>,
) -> Result<ChapterSceneSummary, String> {
    let reporter = |current, total, message: &str| {
        if let Some(progress) = progress {
            progress.progress(current, total, message);
            progress
                .check_cancelled("场景生成已停止")
                .map_err(a4n_core::error::AppError::InvalidArgument)?;
        }
        Ok(())
    };
    chapter_scene_actions::generate_chapter_scene(state, user_id, input, &reporter)
        .await
        .map_err(|err| err.to_string())
}

#[tauri::command]
pub async fn preview_chapter_scene_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: GenerateChapterSceneInput,
) -> Result<ModelRequestPreview, String> {
    chapter_scene_actions::preview_chapter_scene_request(&state, user_id, &input)
        .await
        .map_err(|err| err.to_string())
}
