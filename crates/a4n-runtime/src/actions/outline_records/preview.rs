use crate::AppState;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use a4n_error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::outline::OutlineRequestPreviewStep;

pub(super) async fn outline_preview(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<OutlineRequestPreviewStep>,
) -> Result<ModelRequestPreview, AppError> {
    chat_status_preview_for_state(
        state,
        workflow,
        title,
        steps.into_iter().map(outline_preview_step),
        "llm_chat",
    )
    .await
    .map_err(AppError::InvalidArgument)
}

pub(super) async fn single_request_preview(
    state: &AppState,
    workflow: impl Into<String>,
    title: impl Into<String>,
    key: impl Into<String>,
    title_step: impl Into<String>,
    prompt_name: impl Into<String>,
    request: ChatRequest,
) -> Result<ModelRequestPreview, AppError> {
    outline_preview(
        state,
        workflow,
        title,
        vec![OutlineRequestPreviewStep {
            key: key.into(),
            title: title_step.into(),
            prompt_name: prompt_name.into(),
            state: "ready".to_string(),
            note: None,
            request: Some(request),
        }],
    )
    .await
}

fn outline_preview_step(step: OutlineRequestPreviewStep) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        step.key,
        step.title,
        Some(step.prompt_name),
        step.state,
        step.note,
        step.request,
    )
}
