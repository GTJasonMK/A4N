use super::PreviewInspirationRequestInput;
use crate::AppState;
use crate::actions::access;
use crate::actions::request_preview::{
    ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state, model_request_preview,
};
use a4n_core::error::AppError;

pub async fn preview_inspiration_request(
    state: &AppState,
    user_id: i64,
    input: PreviewInspirationRequestInput,
) -> Result<ModelRequestPreview, AppError> {
    access::verify_project_editor(state, user_id, input.project_id).await?;
    let request = state
        .orchestrator()
        .prepare_inspiration_request_validated(input.project_id, &input.message, state)
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
