use crate::{
    actions::request_preview::{
        ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
    },
    state::AppState,
};

use super::{
    generation::preflight_generate_chapter_analysis, resume::preflight_resume_chapter_analysis_sync,
};

pub async fn preview_chapter_analysis_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    preflight_generate_chapter_analysis(state, user_id, chapter_id).await?;
    let prompt_renderer = state;
    let (chapter, steps) = state
        .orchestrator()
        .prepare_chapter_analysis_preview_validated(chapter_id, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    chat_status_preview_for_state(
        state,
        "chapter_analysis",
        format!("第 {} 章结构化分析请求预览", chapter.chapter_number),
        steps.into_iter().map(preview_step),
        "llm_chat",
    )
    .await
}

pub async fn preview_resume_chapter_analysis_sync_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    preflight_resume_chapter_analysis_sync(state, user_id, chapter_id).await?;
    let prompt_renderer = state;
    let (chapter, steps) = state
        .orchestrator()
        .prepare_resume_chapter_analysis_preview_validated(chapter_id, prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    chat_status_preview_for_state(
        state,
        "chapter_analysis_resume_sync",
        format!("第 {} 章分析继续同步请求预览", chapter.chapter_number),
        steps.into_iter().map(preview_step),
        "llm_chat",
    )
    .await
}

fn preview_step(
    step: a4n_orchestrator::chapter_analysis::ChapterAnalysisPreviewStep,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        step.key,
        step.title,
        step.prompt_name,
        step.status,
        step.note,
        step.request,
    )
}
