use crate::routes::commands::{ApiError, strict_command_input, string_action_result};
use crate::state::ServerState;
use a4n_runtime::host_api::shooting_script_records as shooting_script_records_actions;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn get_chapter_shooting_script(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::ChapterShootingScriptSourceInput =
        strict_command_input(args, "get_chapter_shooting_script")?;
    Ok(json!(string_action_result(
        shooting_script_records_actions::get_chapter_shooting_script(
            state.runtime(),
            user_id,
            input
        )
        .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn get_chapter_shooting_script_unit_progress(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::ChapterShootingScriptSourceInput =
        strict_command_input(args, "get_chapter_shooting_script_unit_progress")?;
    Ok(json!(string_action_result(
        shooting_script_records_actions::get_chapter_shooting_script_unit_progress(
            state.runtime(),
            user_id,
            input,
        )
        .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn delete_chapter_shooting_script(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::DeleteChapterShootingScriptInput =
        strict_command_input(args, "delete_chapter_shooting_script")?;
    Ok(json!(string_action_result(
        shooting_script_records_actions::delete_chapter_shooting_script(
            state.runtime(),
            user_id,
            input,
        )
        .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn edit_chapter_shooting_script(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::EditChapterShootingScriptInput =
        strict_command_input(args, "edit_chapter_shooting_script")?;
    Ok(json!(string_action_result(
        shooting_script_records_actions::edit_chapter_shooting_script(
            state.runtime(),
            user_id,
            input
        )
        .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn edit_chapter_shooting_script_progress(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::EditChapterShootingScriptProgressInput =
        strict_command_input(args, "edit_chapter_shooting_script_progress")?;
    Ok(json!(string_action_result(
        shooting_script_records_actions::edit_chapter_shooting_script_progress(
            state.runtime(),
            user_id,
            input,
        )
        .await,
        false,
    )?))
}
