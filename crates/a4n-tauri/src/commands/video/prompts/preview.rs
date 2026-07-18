use super::input::video_prompt_input;
use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::video_prompts::{
    self as video_prompt_actions, GeneratePromptOptionsInput,
};
use tauri::State;

#[tauri::command]
pub async fn preview_chapter_video_frame_prompt_requests(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    options: GeneratePromptOptionsInput,
) -> Result<ModelRequestPreview, String> {
    video_prompt_actions::preview_chapter_video_frame_prompt_model_request(
        state.inner(),
        video_prompt_input(user_id, project_id, chapter_id, options),
    )
    .await
}

#[tauri::command]
pub async fn preview_chapter_video_prompt_requests(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    options: GeneratePromptOptionsInput,
) -> Result<ModelRequestPreview, String> {
    video_prompt_actions::preview_chapter_video_prompt_model_request(
        state.inner(),
        video_prompt_input(user_id, project_id, chapter_id, options),
    )
    .await
}
