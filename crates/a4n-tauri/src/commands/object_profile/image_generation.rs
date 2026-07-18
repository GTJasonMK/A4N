use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::image_generation as image_generation_actions;
use image_generation_actions::{
    GeneratedImageSummary, ObjectProfileImageInput, ObjectProfileImagePromptOutput,
};
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

tauri_image_progress_input_command!(
    generate_object_profile_image(input: ObjectProfileImageInput) -> GeneratedImageSummary,
    "准备生成物体素材图",
    "物体素材图生成完成",
    |state, user_id, progress| image_generation_actions::object_profile_image_generation_task_scope(
        state.inner(),
        user_id,
        &input,
    )
    .await?,
    image_generation_actions::generate_object_profile_image(
        state.inner(),
        user_id,
        input,
        Some(progress),
    ),
);

#[tauri::command]
pub async fn preview_object_profile_image_request(
    state: State<'_, AppState>,
    user_id: i64,
    input: ObjectProfileImageInput,
) -> Result<ModelRequestPreview, String> {
    image_generation_actions::preview_object_profile_image_request(state.inner(), user_id, input)
        .await
}
