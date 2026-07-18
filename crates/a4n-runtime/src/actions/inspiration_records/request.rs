use super::contracts::{INSPIRATION_VARIABLES_ERROR, PreviewInspirationRequestInput};
use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::inspiration as inspiration_orchestrator;
use serde_json::Value;
use std::collections::HashMap;

pub(super) async fn build_inspiration_request_for_runtime(
    state: &AppState,
    project_id: i64,
    user_input: &str,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    let history = state
        .orchestrator
        .inspiration_messages(project_id, 20)
        .await?;
    let project_context_rows = state
        .orchestrator
        .inspiration_project_context_rows(project_id)
        .await?;
    let prompt_renderer = RuntimePromptRenderer::new(state);
    inspiration_orchestrator::build_inspiration_request_from_context(
        project_id,
        user_input,
        &prompt_renderer,
        variables,
        &history,
        project_context_rows,
    )
    .await
}

pub async fn preview_inspiration_request(
    state: &AppState,
    user_id: i64,
    input: PreviewInspirationRequestInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        INSPIRATION_VARIABLES_ERROR,
    )?;
    let request = build_inspiration_request_for_runtime(
        state,
        input.project_id,
        &input.user_input,
        &variables,
    )
    .await?;
    Ok(model_request_preview(
        "inspiration_conversation",
        "灵感对话请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "inspiration".to_string(),
                    title: "本次灵感对话".to_string(),
                    prompt_name: Some("inspiration".to_string()),
                    state: "ready".to_string(),
                    note: None,
                    request,
                },
            )
            .await
            .map_err(AppError::InvalidArgument)?,
        ],
    ))
}
