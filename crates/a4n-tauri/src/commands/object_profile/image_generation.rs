use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use image_generation_actions::{ObjectProfileImageInput, ObjectProfileImagePromptOutput};
use tauri::State;

#[tauri::command]
pub async fn generate_object_profile_image_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ObjectProfileImagePromptOutput, String> {
    image_generation_actions::generate_object_profile_image_prompt(state.inner(), user_id, input)
        .await
}

#[tauri::command]
pub async fn preview_object_profile_image_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_object_profile_image_request(state.inner(), user_id, input)
        .await
}
