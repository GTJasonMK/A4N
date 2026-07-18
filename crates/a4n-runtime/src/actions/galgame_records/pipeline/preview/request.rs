use super::super::contracts::GALGAME_PIPELINE_MAX_TOKENS_PROFILE;
use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreviewStep, chat_request_step_for_state,
};
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_llm::provider::{ChatMessage, ChatRequest};
use a4n_prompt::rendering::PromptRenderer;
use std::collections::HashMap;

pub(super) async fn galgame_preview_step(
    state: &AppState,
    key: &str,
    title: &str,
    prompt_name: &str,
    user_prompt: String,
) -> Result<ModelRequestPreviewStep, String> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    let system_prompt = prompt_renderer
        .render_prompt(prompt_name, &HashMap::new(), None)
        .await
        .map_err(|err| err.to_string())?;
    let request = ChatRequest::new(
        vec![
            ChatMessage::system(system_prompt),
            ChatMessage::user(user_prompt),
        ],
        Some(0.25),
        None,
        Some(GALGAME_PIPELINE_MAX_TOKENS_PROFILE.to_string()),
    )
    .with_json_response_format();
    chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
}
