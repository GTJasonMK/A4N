use super::context::reverse_content_sample;
use super::contracts::{REVERSE_VARIABLES_ERROR, ReverseEngineerInput};
use super::target::ReverseEngineeringTarget;
use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use serde_json::Value;
use std::collections::HashMap;

pub(super) async fn build_reverse_request_for_user(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<ChatRequest, AppError> {
    super::api::reverse_engineer_task_scope(state, user_id, input, false).await?;
    let variables = parse_template_variables_app_error_from_ref_with_message(
        input.variables.as_ref(),
        REVERSE_VARIABLES_ERROR,
    )?;
    build_reverse_request(state, input, target, &variables).await
}

pub async fn preview_reverse_engineer_request(
    state: &AppState,
    user_id: i64,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
) -> Result<ModelRequestPreview, AppError> {
    let request = build_reverse_request_for_user(state, user_id, input, target).await?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: target.prompt_name().to_string(),
            title: target.preview_step_title().to_string(),
            prompt_name: Some(target.prompt_name().to_string()),
            state: "ready".to_string(),
            note: None,
            request,
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        target.prompt_name(),
        target.preview_title(),
        vec![step],
    ))
}

pub(super) async fn build_reverse_request(
    state: &AppState,
    input: &ReverseEngineerInput,
    target: ReverseEngineeringTarget,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    let content_sample = reverse_content_sample(state, input, target).await?;
    build_reverse_request_from_sample(state, input.project_id, target, &content_sample, variables)
        .await
}

pub(super) async fn reverse_engineer_with_target(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
    content_sample: &str,
    variables: &HashMap<String, Value>,
    workflow_run_id: Option<i64>,
) -> Result<Value, AppError> {
    let request =
        build_reverse_request_from_sample(state, project_id, target, content_sample, variables)
            .await?;
    state
        .reverse_engineer_request(target.prompt_name(), request, workflow_run_id)
        .await
}

pub(crate) async fn build_reverse_request_from_sample(
    state: &AppState,
    project_id: i64,
    target: ReverseEngineeringTarget,
    content_sample: &str,
    variables: &HashMap<String, Value>,
) -> Result<ChatRequest, AppError> {
    let prompt_renderer = RuntimePromptRenderer::new(state);
    a4n_orchestrator::import_reverse::request::build_reverse_engineer_request(
        a4n_orchestrator::import_reverse::request::ReverseEngineerRequestInput {
            project_id,
            prompt_name: target.prompt_name(),
            user_prefix: target.user_instruction(),
            content_sample,
            prompt_renderer: &prompt_renderer,
            variables,
            max_tokens_profile: target.max_tokens_profile(),
        },
    )
    .await
}
