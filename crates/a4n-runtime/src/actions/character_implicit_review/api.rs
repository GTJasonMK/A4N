use super::{
    CheckImplicitUpdateInput, GetImplicitReviewInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};

pub async fn get_implicit_review(
    state: &AppState,
    user_id: i64,
    input: GetImplicitReviewInput,
) -> Result<ImplicitReviewSummary, String> {
    state
        .orchestrator()
        .implicit_review_summary_validated(user_id, input)
        .await
        .map_err(|error| error.to_string())
}

pub async fn check_implicit_update(
    state: &AppState,
    user_id: i64,
    input: CheckImplicitUpdateInput,
) -> Result<ImplicitUpdateDecisionSummary, String> {
    let prompt_renderer = state;
    state
        .orchestrator()
        .check_implicit_update_validated(user_id, input, prompt_renderer)
        .await
        .map_err(|error| error.to_string())
}

pub async fn preview_check_implicit_update_request(
    state: &AppState,
    user_id: i64,
    input: &CheckImplicitUpdateInput,
) -> Result<ModelRequestPreview, String> {
    let prompt_renderer = state;
    let preview = state
        .orchestrator()
        .prepare_implicit_update_review_request_preview_validated(
            user_id,
            input.clone(),
            prompt_renderer,
        )
        .await
        .map_err(|error| error.to_string())?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: "implicit_update_review".to_string(),
            title: "隐性属性复审 LLM 请求".to_string(),
            prompt_name: Some("implicit_update".to_string()),
            state: "ready".to_string(),
            note: None,
            request: preview.request,
        },
    )
    .await?;
    Ok(model_request_preview(
        "implicit_attribute_update_review",
        "隐性属性复审 LLM 请求预览",
        vec![step],
    ))
}
