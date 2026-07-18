use serde_json::Value;

use super::super::super::{
    ApiError, ServerState, command_input, downstream_strategy_arg, i64_arg, manga_records_actions,
    to_value,
};

pub(in crate::routes::commands) async fn list_manga_panels(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    let panels =
        manga_records_actions::list_manga_panels(state.runtime(), user_id, project_id, chapter_id)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(panels)
}

pub(in crate::routes::commands) async fn bind_manga_reference_image(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: manga_records_actions::BindReferenceImageInput =
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
    let prompt_id = i64_arg(args, "promptId")?;
    let downstream_strategy = downstream_strategy_arg(args)?;
    manga_records_actions::delete_manga_prompt(
        state.runtime(),
        user_id,
        prompt_id,
        downstream_strategy,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn delete_manga_by_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_id = i64_arg(args, "chapterId")?;
    manga_records_actions::delete_manga_by_chapter(
        state.runtime(),
        user_id,
        project_id,
        chapter_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}
