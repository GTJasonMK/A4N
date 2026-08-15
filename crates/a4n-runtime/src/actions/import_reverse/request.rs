use super::{ReverseEngineerInput, authorize_reverse_engineer};
use crate::AppState;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;
use a4n_orchestrator::import_reverse::ReverseEngineerPreviewInput;

async fn build_reverse_request_for_user(
    state: &AppState,
    user_id: i64,
    input: ReverseEngineerInput,
) -> Result<(super::ReverseEngineeringTarget, ChatRequest), AppError> {
    let input = authorize_reverse_engineer(state, user_id, input)
        .await?
        .into_input();
    let target = input.target;
    let variables = input.variables.unwrap_or_default();
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .prepare_reverse_engineer_request_validated(ReverseEngineerPreviewInput {
            project_id: input.project_id,
            manual_sample: &input.content_sample,
            auto_context: input.auto_context,
            context_limit: input.context_limit,
            variables: &variables,
            target,
            prompt_renderer,
        })
        .await?;
    Ok((target, request))
}

pub async fn preview_reverse_engineer_request(
    state: &AppState,
    user_id: i64,
    input: ReverseEngineerInput,
) -> Result<ModelRequestPreview, AppError> {
    let (target, request) = build_reverse_request_for_user(state, user_id, input).await?;
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
