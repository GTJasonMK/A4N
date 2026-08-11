use super::contracts::{INSPIRATION_VARIABLES_ERROR, PreviewInspirationRequestInput};
use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use crate::actions::template_variables::parse_template_variables_app_error_from_ref_with_message;
use a4n_core::error::AppError;
use a4n_orchestrator::inspiration::InspirationPreviewInput;

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
    let prompt_renderer = state;
    let request = state
        .orchestrator()
        .prepare_inspiration_request_validated(InspirationPreviewInput {
            project_id: input.project_id,
            user_input: &input.user_input,
            variables: &variables,
            prompt_renderer: prompt_renderer,
        })
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
