use a4n_runtime::AppState;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use image_generation_actions::{SceneProfileImageInput, SceneProfileImagePromptOutput};
use tauri::State;

#[tauri::command]
pub async fn generate_scene_profile_image_prompt(
    state: State<'_, AppState>,
    user_id: i64,
    input: SceneProfileImageInput,
) -> Result<SceneProfileImagePromptOutput, String> {
    image_generation_actions::generate_scene_profile_image_prompt(state.inner(), user_id, input)
        .await
}
