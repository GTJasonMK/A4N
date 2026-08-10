use a4n_core::error::AppError;
use a4n_runtime::host_api::task_records as task_records_actions;

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
    task_id: Option<String>,
    result: Result<T, AppError>,
) -> Result<T, ApiError> {
    if let Some(task_id) = task_id.as_deref() {
        task_records_actions::remove_task_handle(state.runtime(), task_id);
    }
    result.map_err(ApiError::from)
}
