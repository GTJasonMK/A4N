use super::ReverseEngineeringTarget;
use super::contracts::{REVERSE_VARIABLES_ERROR, ReverseEngineerInput};
use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use crate::prompt_renderer::RuntimePromptRenderer;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::import_reverse::ReverseEngineerPreviewInput;

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
    let prompt_renderer = RuntimePromptRenderer::new(state);
    state
        .import_reverse()
        .prepare_reverse_engineer_request_validated(ReverseEngineerPreviewInput {
            project_id: input.project_id,
            manual_sample: &input.content_sample,
            auto_context: input.auto_context,
            context_limit: input.context_limit,
            variables: &variables,
            target,
            prompt_renderer: &prompt_renderer,
        })
        .await
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
