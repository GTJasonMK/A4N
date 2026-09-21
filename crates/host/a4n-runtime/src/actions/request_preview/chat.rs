use super::{ChatRequestStepInput, ModelRequestPreview, ModelRequestPreviewStep};
use crate::AppState;
use a4n_llm_provider_contract::provider::ChatRequest;

mod budget;
mod step;

pub(crate) use budget::chat_request_step_for_state;
pub(crate) use step::chat_request_step_with_budget;

pub(crate) async fn traced_chat_request_step_for_state(
    state: &AppState,
    request: ChatRequest,
    title: &str,
    prompt_name: &str,
    missing_call_key_message: &str,
) -> Result<ModelRequestPreviewStep, String> {
    let key = request
        .trace_context
        .as_ref()
        .and_then(|context| context.call_key.clone())
        .ok_or_else(|| missing_call_key_message.to_string())?;
    chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key,
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
}

pub(crate) fn model_request_preview(
    workflow: impl Into<String>,
    title: impl Into<String>,
    steps: Vec<ModelRequestPreviewStep>,
) -> ModelRequestPreview {
    let ready_step_count = steps.iter().filter(|step| step.state == "ready").count();
    let total_chars = steps.iter().map(|step| step.total_chars).sum();
    ModelRequestPreview {
        workflow: workflow.into(),
        title: title.into(),
        steps,
        ready_step_count,
        total_chars,
    }
}
