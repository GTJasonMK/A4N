use a4n_runtime::AppState;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use image_generation_actions::{
    GeneratedImageSummary, SceneProfileImageInput, SceneProfileImagePromptOutput,
};
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

tauri_image_progress_input_command!(
    generate_scene_profile_image(input: SceneProfileImageInput) -> GeneratedImageSummary,
    "准备生成场景素材图",
    "场景素材图生成完成",
    |state, user_id, progress| image_generation_actions::scene_profile_image_generation_task_scope(
        state.inner(),
        user_id,
        &input,
    )
    .await?,
    image_generation_actions::generate_scene_profile_image(
        state.inner(),
        user_id,
        input,
        Some(progress),
    ),
);
