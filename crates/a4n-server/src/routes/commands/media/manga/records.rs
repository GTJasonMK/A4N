use serde_json::Value;

use super::super::super::{ApiError, ServerState, command_input, manga_records_actions, to_value};

pub(in crate::routes::commands) async fn list_manga_page_prompts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::MangaChapterInput =
        command_input(args, "list_manga_page_prompts")?;
    let page_prompts =
        manga_records_actions::list_manga_page_prompts(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(page_prompts)
}

pub(in crate::routes::commands) async fn bind_manga_reference_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::BindMangaReferenceImageInput =
        command_input(args, "bind_manga_reference_image")?;
    let updated =
        manga_records_actions::bind_manga_reference_image(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn update_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::UpdateMangaPromptInput =
        command_input(args, "update_manga_prompt")?;
    let updated = manga_records_actions::update_manga_prompt(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn optimize_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::OptimizeMangaPromptInput =
        command_input(args, "optimize_manga_prompt")?;
    let updated = manga_records_actions::optimize_manga_prompt(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(updated)
}

pub(in crate::routes::commands) async fn delete_manga_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::DeleteMangaPromptInput =
        command_input(args, "delete_manga_prompt")?;
    manga_records_actions::delete_manga_prompt(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn delete_manga_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::MangaChapterInput =
        command_input(args, "delete_manga_by_chapter")?;
    manga_records_actions::delete_manga_by_chapter(state.runtime(), user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
