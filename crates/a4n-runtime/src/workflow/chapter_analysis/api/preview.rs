use crate::{
    actions::request_preview::{
        ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
    },
    state::AppState,
};

pub async fn preview_chapter_analysis_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let preview = state
        .chapter_analysis_preview_application()
        .preview(user_id, chapter_id, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    chat_status_preview_for_state(
        state,
        "chapter_analysis",
        preview.title,
        preview.steps.into_iter().map(preview_step),
        "llm_chat",
    )
    .await
}

pub async fn preview_resume_chapter_analysis_sync_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let preview = state
        .chapter_analysis_preview_application()
        .preview_resume(user_id, chapter_id, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    chat_status_preview_for_state(
        state,
        "chapter_analysis_resume_sync",
        preview.title,
        preview.steps.into_iter().map(preview_step),
        "llm_chat",
    )
    .await
}

fn preview_step(
    step: a4n_prompt::chapter_analysis::ChapterAnalysisRequestStep,
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
