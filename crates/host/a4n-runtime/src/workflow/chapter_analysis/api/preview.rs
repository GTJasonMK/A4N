use crate::{
    actions::request_preview::{
        ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
    },
    state::AppState,
};
use a4n_error::AppError;

pub async fn preview_chapter_analysis_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let preview = state
        .chapter_analysis_generation_preview_application()
        .preview(user_id, chapter_id, prompt_renderer)
        .await?;
    chat_status_preview_for_state(
        state,
        "chapter_analysis",
        preview.title,
        preview.steps.into_iter().map(preview_step),
        "llm_chat",
    )
    .await
    .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
}

pub async fn preview_resume_chapter_analysis_sync_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let preview = state
        .chapter_analysis_resume_preview_application()
        .preview_resume(user_id, chapter_id, prompt_renderer)
        .await?;
    chat_status_preview_for_state(
        state,
        "chapter_analysis_resume_sync",
        preview.title,
        preview.steps.into_iter().map(preview_step),
        "llm_chat",
    )
    .await
    .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
}

fn preview_step(
    step: a4n_chapter_analysis_host_contract::ChapterAnalysisRequestStep,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        step.key,
        step.title,
        Some(step.prompt_name),
        "ready",
        None,
        Some(step.request),
    )
}
