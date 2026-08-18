use std::sync::Arc;

use crate::AppState;
use a4n_core::{error::AppError, event::channel::TaskHandle};

pub fn register_task_handle(
    state: &AppState,
    handle: TaskHandle,
) -> Result<Arc<TaskHandle>, AppError> {
    state.orchestrator().register_task_handle(handle)
}

pub fn remove_task_handle(state: &AppState, handle: &Arc<TaskHandle>) -> Result<(), AppError> {
    state.orchestrator().remove_task_handle(handle)
}

pub fn cancel_owned_task_handle(state: &AppState, task_id: &str, user_id: i64) -> bool {
    state
        .orchestrator()
        .cancel_owned_task_handle(task_id, user_id)
}
