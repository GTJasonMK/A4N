use crate::{
    AppState,
    actions::request_preview::{
        ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
        model_request_preview,
    },
};
use a4n_core::error::AppError;
use a4n_orchestrator::character::CharacterConflictCheckInput;

pub async fn preview_character_conflict_request(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<ModelRequestPreview, AppError> {
    let prompt_renderer = state;
    let preview = state
        .orchestrator()
        .prepare_character_conflict_request_preview_validated(
            user_id,
            CharacterConflictCheckInput {
                project_id,
                name: name.to_string(),
                start_chapter,
                end_chapter,
            },
            prompt_renderer,
        )
        .await?;
    let step = chat_request_step_for_state(
        state,
        ChatRequestStepInput {
            key: "character_conflict".to_string(),
            title: "角色一致性检查 LLM 请求".to_string(),
            prompt_name: Some("character_conflict_check".to_string()),
            state: "ready".to_string(),
            note: None,
            request: preview.request,
        },
    )
    .await
    .map_err(AppError::InvalidArgument)?;
    Ok(model_request_preview(
        "character_conflict_check",
        "角色一致性检查 LLM 请求预览",
        vec![step],
    ))
}
