use std::sync::Arc;

use a4n_error::AppError;
use a4n_runtime::host_api::task_records as task_records_actions;
use a4n_task_runtime::channel::TaskHandle;

use super::super::{ApiError, ServerState};

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

pub(in crate::routes::commands) fn finish_web_task<T>(
    state: &ServerState,
    task: Option<&Arc<TaskHandle>>,
    result: Result<T, AppError>,
) -> Result<T, ApiError> {
    if let Some(task) = task {
        task_records_actions::remove_task_handle(state.runtime(), task)?;
    }
    result.map_err(ApiError::from)
}
