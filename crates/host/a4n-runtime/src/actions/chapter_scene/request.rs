use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};
use a4n_api_contract::chapter::GenerateChapterSceneInput;
use a4n_error::AppError;

pub async fn preview_chapter_scene_request(
    state: &AppState,
    user_id: i64,
    input: &GenerateChapterSceneInput,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let preview = state
        .chapter_scene_generation_application()
        .preview_for_user(user_id, input.clone(), prompt_renderer)
        .await?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: "chapter_scene_generation".to_string(),
            title: format!(
                "第 {} 章 · 场景 {}",
                preview.chapter_number, input.scene_number
            ),
            prompt_name: Some("chapter_scene_generation".to_string()),
            prompt_assembly: None,
            state: "ready".to_string(),
            note: None,
            request: preview.request,
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        "chapter_scene_generation",
        format!("第 {} 章场景生成请求预览", preview.chapter_number),
        vec![step],
    ))
}
