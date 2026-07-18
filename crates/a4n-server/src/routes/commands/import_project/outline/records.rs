use crate::state::ServerState;
use serde_json::Value;

use super::super::super::{
    ApiError, command_input, i64_arg, outline_records_actions, to_value, u64_arg,
};

pub(in crate::routes::commands) async fn list_parts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(outline_records_actions::list_parts(state.runtime(), user_id, project_id).await?)
}

pub(in crate::routes::commands) async fn delete_latest_parts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let count = u64_arg(args, "count")?;
    outline_records_actions::delete_latest_parts(state.runtime(), user_id, project_id, count)
        .await?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn get_generation_progress(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        outline_records_actions::get_generation_progress(state.runtime(), user_id, project_id)
            .await?,
    )
}

pub(in crate::routes::commands) async fn list_chapter_outlines(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(
        outline_records_actions::list_chapter_outlines(state.runtime(), user_id, project_id)
            .await?,
    )
}

pub(in crate::routes::commands) async fn delete_latest_chapter_outlines(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let count = u64_arg(args, "count")?;
    outline_records_actions::delete_latest_chapter_outlines(
        state.runtime(),
        user_id,
        project_id,
        count,
    )
    .await?;
    Ok(Value::Null)
}

pub(in crate::routes::commands) async fn update_chapter_outline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: outline_records_actions::UpdateChapterOutlineInput =
        command_input(args, "update_chapter_outline")?;
    to_value(
        outline_records_actions::update_chapter_outline(state.runtime(), user_id, input).await?,
    )
}

pub(in crate::routes::commands) async fn get_chapter_outlines_pacing(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(
        outline_records_actions::get_chapter_outlines_pacing(state.runtime(), user_id, project_id)
            .await?,
    )
}
