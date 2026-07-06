use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep, blocked_step,
    chat_request_step_for_state, image_provider_request_step, model_request_preview,
};
use crate::state::AppState;
use a4n_engine::image::ImageGenerationRequest;
use a4n_engine::llm::provider::ChatRequest;

pub(super) fn portrait_preview(
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<ModelRequestPreviewStep>,
) -> Result<ModelRequestPreview, String> {
    Ok(model_request_preview(workflow, title, steps))
}

pub(super) async fn chat_preview_step(
    state: &AppState,
    key: &str,
    title: &str,
    prompt_name: Option<&str>,
    request: ChatRequest,
) -> Result<ModelRequestPreviewStep, String> {
    chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: prompt_name.map(str::to_string),
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
}

pub(super) fn provider_preview_step(
    key: &str,
    title: &str,
    backend: &str,
    request: ImageGenerationRequest,
) -> Result<ModelRequestPreviewStep, String> {
    image_provider_request_step(
        key.to_string(),
        title.to_string(),
        None,
        "ready",
        None,
        backend,
        request,
        None,
    )
}

pub(super) fn blocked_preview_step(
    key: &str,
    title: &str,
    kind: &str,
    note: &str,
) -> ModelRequestPreviewStep {
    blocked_step(
        key.to_string(),
        title.to_string(),
        kind.to_string(),
        None,
        note.to_string(),
    )
}
