use crate::routes::commands::{ApiError, chapter_content_actions, command_input, i64_arg};
use crate::state::ServerState;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn list_chapter_analyses(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        chapter_content_actions::list_chapter_analyses(state.runtime(), user_id, project_id)
            .await?
    ))
}

pub(in crate::routes::commands) async fn list_story_event_chapters(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        chapter_content_actions::list_story_event_chapters(state.runtime(), user_id, project_id)
            .await?
    ))
}

pub(in crate::routes::commands) async fn list_chapter_index(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        chapter_content_actions::list_chapter_index(state.runtime(), user_id, project_id).await?
    ))
}

pub(in crate::routes::commands) async fn get_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_content_actions::GetChapterInput = command_input(args, "get_chapter")?;
    Ok(json!(
        chapter_content_actions::get_chapter(
            state.runtime(),
            user_id,
            input.project_id,
            input.chapter_id
        )
        .await?
    ))
}

pub(in crate::routes::commands) async fn list_chapters(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        chapter_content_actions::list_chapters(state.runtime(), user_id, project_id).await?
    ))
}

pub(in crate::routes::commands) async fn import_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_content_actions::ImportChapterInput = command_input(args, "import_chapter")?;
    Ok(json!(
        chapter_content_actions::import_chapter(state.runtime(), user_id, input).await?
    ))
}

pub(in crate::routes::commands) async fn update_chapter_content(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_content_actions::UpdateChapterContentInput =
        command_input(args, "update_chapter_content")?;
    Ok(json!(
        chapter_content_actions::update_chapter_content(state.runtime(), user_id, input).await?
    ))
}
