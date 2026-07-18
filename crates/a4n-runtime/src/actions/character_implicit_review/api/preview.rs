use super::super::{
    CheckImplicitUpdateInput, IMPLICIT_UPDATE_PROMPT_NAME,
    request::build_check_implicit_update_request,
};
use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};

pub async fn preview_check_implicit_update_request(
    state: &AppState,
    user_id: i64,
    input: &CheckImplicitUpdateInput,
) -> Result<ModelRequestPreview, String> {
    let context = build_check_implicit_update_request(state, user_id, input).await?;
    Ok(model_request_preview(
        "implicit_attribute_update_review",
        "隐性属性复审 LLM 请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "implicit_update_review".to_string(),
                    title: "隐性属性复审 LLM 请求".to_string(),
                    prompt_name: Some(IMPLICIT_UPDATE_PROMPT_NAME.to_string()),
                    state: "ready".to_string(),
                    note: None,
                    request: context.request,
                },
            )
            .await?,
        ],
    ))
}
