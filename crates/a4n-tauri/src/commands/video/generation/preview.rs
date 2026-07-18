use crate::commands::request_preview::ModelRequestPreview;
use a4n_runtime::AppState;
use a4n_runtime::host_api::video_generation::{
    self as video_generation_actions, PreviewChapterVideoPromptOverrideInput,
    ReferenceMediaVideoInput,
};
use tauri::State;

#[tauri::command]
pub async fn preview_chapter_video_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
    config_id: Option<i64>,
    prompt_override: Option<PreviewChapterVideoPromptOverrideInput>,
) -> Result<ModelRequestPreview, String> {
    video_generation_actions::preview_chapter_video_request(
        state.inner(),
        video_generation_actions::GenerateChapterVideoInput {
            user_id,
            project_id,
            chapter_id,
            prompt_id,
            config_id,
        },
        prompt_override,
    )
    .await
}

#[tauri::command]
pub async fn preview_video_provider_request_by_video_id(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    video_id: i64,
) -> Result<ModelRequestPreview, String> {
    video_generation_actions::preview_video_provider_request_by_video_id(
        state.inner(),
        user_id,
        project_id,
        chapter_id,
        video_id,
    )
    .await
}

#[tauri::command]
pub async fn preview_reference_media_video_request(
    state: State<'_, AppState>,
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    input: ReferenceMediaVideoInput,
) -> Result<ModelRequestPreview, String> {
    video_generation_actions::preview_reference_media_video_request(
        state.inner(),
        video_generation_actions::GenerateReferenceMediaVideoInput {
            user_id,
            project_id,
            chapter_id,
            input,
        },
    )
    .await
}
