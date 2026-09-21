use crate::routes::commands::{ApiError, chapter_content_actions, command_input, i64_arg};
use crate::state::ServerState;
use serde_json::{Value, json};

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
