use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use tauri::State;

#[tauri::command]
pub async fn preview_scene_image_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    prompt: String,
    config_id: Option<i64>,
    chapter_id: Option<i64>,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_scene_image_request(
        state.inner(),
        user_id,
        project_id,
        prompt,
        config_id,
        chapter_id,
    )
    .await
}

#[tauri::command]
pub async fn preview_page_image_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_number: i32,
    page_number: i32,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_page_image_request(
        state.inner(),
        user_id,
        project_id,
        chapter_number,
        page_number,
    )
    .await
}

#[tauri::command]
pub async fn preview_batch_images_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    requests_json: serde_json::Value,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_batch_images_request(
        state.inner(),
        user_id,
        project_id,
        requests_json,
    )
    .await
}
