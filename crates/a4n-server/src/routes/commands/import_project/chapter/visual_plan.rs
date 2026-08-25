use std::sync::Arc;

use a4n_core::event::channel::{TaskHandle, TaskKind};
use a4n_runtime::host_api::chapter_visual_plan_records as visual_plan_actions;
use serde_json::{Value, json};

use crate::routes::commands::{
    ApiError, finish_web_task, optional_string_field, register_web_task, sanitize_optional_task_id,
    strict_command_input, strict_optional_task_runtime_command_input, string_action_result,
    web_progress_reporter, web_stream_reporter,
};
use crate::state::ServerState;

pub(in crate::routes::commands) async fn get_chapter_visual_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "get_chapter_visual_plan")?;
    Ok(json!(string_action_result(
        visual_plan_actions::get_chapter_visual_plan(state.runtime(), user_id, input).await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn save_chapter_visual_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "save_chapter_visual_plan_draft")?;
    Ok(json!(string_action_result(
        visual_plan_actions::save_chapter_visual_plan_draft(state.runtime(), user_id, input).await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn apply_chapter_visual_plan_proposal(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "apply_chapter_visual_plan_proposal")?;
    Ok(json!(string_action_result(
        visual_plan_actions::apply_chapter_visual_plan_proposal(state.runtime(), user_id, input)
            .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn confirm_chapter_visual_plan(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "confirm_chapter_visual_plan")?;
    Ok(json!(string_action_result(
        visual_plan_actions::confirm_chapter_visual_plan(state.runtime(), user_id, input).await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn discard_chapter_visual_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "discard_chapter_visual_plan_draft")?;
    Ok(json!(string_action_result(
        visual_plan_actions::discard_chapter_visual_plan_draft(state.runtime(), user_id, input)
            .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn preview_chapter_visual_plan_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input = strict_command_input(args, "preview_chapter_visual_plan_request")?;
    Ok(json!(string_action_result(
        visual_plan_actions::preview_chapter_visual_plan_request(state.runtime(), user_id, input)
            .await,
        false,
    )?))
}

pub(in crate::routes::commands) async fn generate_chapter_visual_plan_draft(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: visual_plan_actions::GenerateChapterVisualPlanDraftInput =
        strict_optional_task_runtime_command_input(args, "generate_chapter_visual_plan_draft")?;
    let (task, cancel, progress, stream) = task_context(state, user_id, args, input.project_id)?;
    let result = visual_plan_actions::generate_chapter_visual_plan_draft(
        state.runtime(),
        user_id,
        input,
        &progress,
        Some(&stream),
        cancel.as_ref(),
    )
    .await;
    finish_task(state, task.as_ref(), result)
}

pub(in crate::routes::commands) async fn propose_chapter_visual_plan_revision(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: visual_plan_actions::ProposeChapterVisualPlanRevisionInput =
        strict_optional_task_runtime_command_input(args, "propose_chapter_visual_plan_revision")?;
    let (task, cancel, progress, stream) = task_context(state, user_id, args, input.project_id)?;
    let result = visual_plan_actions::propose_chapter_visual_plan_revision(
        state.runtime(),
        user_id,
        input,
        &progress,
        Some(&stream),
        cancel.as_ref(),
    )
    .await;
    finish_task(state, task.as_ref(), result)
}

type WebProgress =
    Box<dyn Fn(u32, u32, &str) -> Result<(), a4n_core::error::AppError> + Send + Sync>;
type WebStream = Box<dyn Fn(&str) + Send + Sync>;

fn task_context(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    project_id: i64,
) -> Result<
    (
        Option<Arc<TaskHandle>>,
        Option<tokio_util::sync::CancellationToken>,
        WebProgress,
        WebStream,
    ),
    ApiError,
> {
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        user_id,
        task_id,
        TaskKind::LlmCall,
        project_id,
        visual_plan_actions::CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL,
        "正在准备导演编排",
    )?;
    let cancel = task.as_ref().map(|handle| handle.cancel_token.clone());
    Ok((
        task.clone(),
        cancel,
        Box::new(web_progress_reporter(task.clone())),
        Box::new(web_stream_reporter(task)),
    ))
}

fn finish_task<T: serde::Serialize>(
    state: &ServerState,
    task: Option<&Arc<TaskHandle>>,
    result: Result<T, String>,
) -> Result<Value, ApiError> {
    let result = finish_web_task(
        state,
        task,
        string_action_result(result, task.is_some_and(|handle| handle.is_cancelled())),
    )?;
    Ok(json!(result))
}
