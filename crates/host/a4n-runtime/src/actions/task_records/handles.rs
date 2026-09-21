use std::sync::Arc;

use crate::AppState;
use a4n_error::AppError;
use a4n_task_runtime::channel::TaskHandle;

pub fn register_task_handle(
    state: &AppState,
    handle: TaskHandle,
) -> Result<Arc<TaskHandle>, AppError> {
    state.task_manager().register(handle)
}

pub fn remove_task_handle(state: &AppState, handle: &Arc<TaskHandle>) -> Result<(), AppError> {
    state.task_manager().remove(handle)
}

pub fn cancel_owned_task_handle(state: &AppState, task_id: &str, user_id: i64) -> bool {
    state.task_manager().cancel_owned(task_id, user_id)
}
