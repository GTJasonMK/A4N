use super::build_character_conflict_request;
use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};
use a4n_core::error::AppError;

pub async fn preview_character_conflict_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<ModelRequestPreview, AppError> {
    let context = build_character_conflict_request(
        state,
        user_id,
        project_id,
        name,
        start_chapter,
        end_chapter,
    )
    .await?;
    Ok(model_request_preview(
        "character_conflict_check",
        "角色一致性检查 LLM 请求预览",
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "character_conflict".to_string(),
                    title: "角色一致性检查 LLM 请求".to_string(),
                    prompt_name: Some("character_conflict_check".to_string()),
                    state: "ready".to_string(),
                    note: None,
                    request: context.request,
                },
            )
            .await
            .map_err(AppError::InvalidArgument)?,
        ],
    ))
}
