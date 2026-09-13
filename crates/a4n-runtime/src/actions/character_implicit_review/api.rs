use super::{
    CheckImplicitUpdateInput, GetImplicitReviewInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
use crate::{
    AppState,
    actions::request_preview::{
        ModelRequestPreview, model_request_preview, traced_chat_request_step_for_state,
    },
};
use a4n_orchestrator::character::{IMPLICIT_UPDATE_PROMPT_NAME, IMPLICIT_UPDATE_WORKFLOW_KIND};

pub async fn get_implicit_review(
    state: &AppState,
    user_id: i64,
    input: GetImplicitReviewInput,
) -> Result<ImplicitReviewSummary, String> {
    state
        .character_implicit_review_read_application()
        .summary(user_id, input)
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
        .character_implicit_review_check_application()
        .check(user_id, input, prompt_renderer)
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
        .character_implicit_review_preview_application()
        .preview(user_id, input.clone(), prompt_renderer)
        .await
        .map_err(|error| error.to_string())?;
    let step = traced_chat_request_step_for_state(
        state,
        preview,
        "隐性属性复审 LLM 请求",
        IMPLICIT_UPDATE_PROMPT_NAME,
        "隐性属性复审请求缺少 call_key",
    )
    .await?;
    Ok(model_request_preview(
        IMPLICIT_UPDATE_WORKFLOW_KIND,
        "隐性属性复审 LLM 请求预览",
        vec![step],
    ))
}
