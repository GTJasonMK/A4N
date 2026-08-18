use super::{
    CheckImplicitUpdateInput, GetImplicitReviewInput, ImplicitReviewSummary,
    ImplicitUpdateDecisionSummary,
};
use a4n_orchestrator::character::{IMPLICIT_UPDATE_PROMPT_NAME, IMPLICIT_UPDATE_WORKFLOW_KIND};
use crate::{
    AppState,
    actions::request_preview::{
        ModelRequestPreview, model_request_preview, traced_chat_request_step_for_state,
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
