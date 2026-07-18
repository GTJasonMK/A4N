use a4n_core::error::AppError;
use a4n_runtime::host_api::task_records as task_records_actions;
use a4n_runtime::host_api::video_generation as video_generation_actions;

use super::super::{ApiError, ServerState};
use super::events::{emit_workflow_complete, emit_workflow_error};

pub(in crate::routes::commands) fn string_action_result<T>(
    result: Result<T, String>,
    cancelled: bool,
) -> Result<T, AppError> {
    result.map_err(|message| {
        if cancelled {
            AppError::Cancelled
        } else {
            AppError::InvalidArgument(message)
        }
    })
}

pub(in crate::routes::commands) async fn finish_web_task<T>(
    state: &ServerState,
    task_id: Option<String>,
    result: Result<T, AppError>,
    success_message: &str,
) -> Result<T, ApiError> {
    match result {
        Ok(value) => {
            if let Some(task_id) = task_id.as_deref() {
                emit_workflow_complete(state, task_id, Some(success_message.to_string())).await;
                task_records_actions::remove_task_handle(state.runtime(), task_id);
            }
            Ok(value)
        }
        Err(err) => {
            if let Some(task_id) = task_id.as_deref() {
                let is_cancelled = matches!(err, AppError::Cancelled);
                emit_workflow_error(
                    state,
                    task_id,
                    if is_cancelled {
                        "cancelled"
                    } else {
                        "workflow_failed"
                    },
                    &err.to_string(),
                    false,
                )
                .await;
                task_records_actions::remove_task_handle(state.runtime(), task_id);
            }
            Err(err.into())
        }
    }
}

pub(in crate::routes::commands) async fn finish_web_video_generation_task<F>(
    state: &ServerState,
    task_id: Option<String>,
    result: Result<video_generation_actions::GeneratedVideoSummary, String>,
    cancelled: bool,
    error_code: &str,
    success_summary: F,
) -> Result<video_generation_actions::GeneratedVideoSummary, ApiError>
where
    F: FnOnce(&video_generation_actions::GeneratedVideoSummary) -> String,
{
    match result {
        Ok(video) => {
            if let Some(task_id) = task_id.as_deref() {
                if video.status == "failed" {
                    emit_workflow_error(
                        state,
                        task_id,
                        error_code,
                        &success_summary(&video),
                        false,
                    )
                    .await;
                } else {
                    emit_workflow_complete(state, task_id, Some(success_summary(&video))).await;
                }
                task_records_actions::remove_task_handle(state.runtime(), task_id);
            }
            Ok(video)
        }
        Err(err) => {
            if let Some(task_id) = task_id.as_deref() {
                emit_workflow_error(
                    state,
                    task_id,
                    if cancelled { "cancelled" } else { error_code },
                    &err,
                    false,
                )
                .await;
                task_records_actions::remove_task_handle(state.runtime(), task_id);
            }
            Err(ApiError::bad_request(err))
        }
    }
}
