use crate::routes::commands::{
    ApiError, finish_web_task, optional_string_field, register_web_task, sanitize_optional_task_id,
    strict_command_input, strict_optional_task_runtime_command_input, string_action_result,
    web_progress_reporter, web_stream_reporter,
};
use crate::state::ServerState;
use a4n_core::event::channel::TaskKind;
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

pub(in crate::routes::commands) async fn generate_chapter_shooting_script(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::ChapterShootingScriptSourceInput =
        strict_optional_task_runtime_command_input(args, "generate_chapter_shooting_script")?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        input.project_id,
        shooting_script_records_actions::CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        "正在准备剧本改编",
    )?;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let progress = web_progress_reporter(task.clone());
    let stream_reporter = web_stream_reporter(task.clone());
    let result = shooting_script_records_actions::generate_chapter_shooting_script(
        state.runtime(),
        user_id,
        input,
        &progress,
        Some(&stream_reporter),
        cancel_token.as_ref(),
        task_id.as_deref(),
    )
    .await;
    let result = finish_web_task(
        state,
        task.as_ref(),
        string_action_result(
            result,
            task.as_ref().is_some_and(|handle| handle.is_cancelled()),
        ),
    )?;
    Ok(json!(result))
}

pub(in crate::routes::commands) async fn generate_next_chapter_shooting_script_unit(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: shooting_script_records_actions::ChapterShootingScriptSourceInput =
        strict_optional_task_runtime_command_input(
            args,
            "generate_next_chapter_shooting_script_unit",
        )?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id.clone(),
        TaskKind::LlmCall,
        input.project_id,
        shooting_script_records_actions::CHAPTER_SHOOTING_SCRIPT_PROGRESS_TOTAL,
        "正在准备下一个场景改编",
    )?;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let progress = web_progress_reporter(task.clone());
    let stream_reporter = web_stream_reporter(task.clone());
    let result = shooting_script_records_actions::generate_next_chapter_shooting_script_unit(
        state.runtime(),
        user_id,
        input,
        &progress,
        Some(&stream_reporter),
        cancel_token.as_ref(),
        task_id.as_deref(),
    )
    .await;
    let result = finish_web_task(
        state,
        task.as_ref(),
        string_action_result(
            result,
            task.as_ref().is_some_and(|handle| handle.is_cancelled()),
        ),
    )?;
    Ok(json!(result))
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
