use std::sync::Arc;

use a4n_core::event::channel::{TaskHandle, TaskKind};
use a4n_runtime::host_api::galgame_records as galgame_records_actions;
use serde_json::Value;

use crate::state::ServerState;

use super::super::{
    ApiError, command_input, finish_web_task, register_web_task, sanitize_task_id,
    string_action_result, to_value,
};

fn web_galgame_progress_reporter(
    task: Option<Arc<TaskHandle>>,
) -> impl Fn(
    galgame_records_actions::GalgamePipelineProgressUpdate,
) -> Result<(), a4n_core::error::AppError>
+ Send
+ Sync
+ 'static {
    move |event| {
        if let Some(handle) = task.as_ref() {
            if handle.is_cancelled() {
                return Err(a4n_core::error::AppError::Cancelled);
            }
            handle.update_progress_with_meta(
                event.current,
                event.total,
                &event.message,
                Some(event.meta.clone()),
            );
        }
        Ok(())
    }
}

pub(in crate::routes::commands) async fn start_galgame_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let mut input: galgame_records_actions::StartGalgamePipelineInput =
        command_input(args, "start_galgame_pipeline")?;
    input.task_id = sanitize_task_id(input.task_id)?;
    let task_id = input.task_id.clone();
    let task = register_web_task(
        state,
        user_id,
        Some(task_id.clone()),
        TaskKind::GalgameGeneration,
        Some(input.project_id),
        galgame_records_actions::GALGAME_PIPELINE_TOTAL_STEPS,
        "正在准备 Galgame 管线",
    );
    let progress_reporter = web_galgame_progress_reporter(task.clone());
    let result = string_action_result(
        galgame_records_actions::start_galgame_pipeline_with_progress(
            state.runtime(),
            user_id,
            input,
            Some(Arc::new(progress_reporter)),
        )
        .await,
        task.as_ref().is_some_and(|handle| handle.is_cancelled()),
    );
    to_value(finish_web_task(state, Some(task_id), result)?)
}

pub(in crate::routes::commands) async fn cancel_galgame_pipeline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::cancel_galgame_pipeline(
            state.runtime(),
            user_id,
            command_input(args, "cancel_galgame_pipeline")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn preview_galgame_pipeline_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::preview_galgame_pipeline_request(
            state.runtime(),
            user_id,
            command_input(args, "preview_galgame_pipeline_request")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(in crate::routes::commands) async fn get_galgame_pipeline_status(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        galgame_records_actions::get_galgame_pipeline_status(
            state.runtime(),
            user_id,
            command_input(args, "get_galgame_pipeline_status")?,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}
