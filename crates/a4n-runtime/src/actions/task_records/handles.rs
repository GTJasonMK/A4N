use crate::AppState;
use a4n_core::event::channel::TaskHandle;
use std::sync::Arc;

pub fn register_task_handle(state: &AppState, handle: TaskHandle) -> Arc<TaskHandle> {
    state.register_task_handle(handle)
}

pub fn remove_task_handle(state: &AppState, task_id: &str) -> Option<Arc<TaskHandle>> {
    state.remove_task_handle(task_id)
}

pub fn get_task_handle(state: &AppState, task_id: &str) -> Option<Arc<TaskHandle>> {
    state.task_handle(task_id)
}

pub fn cancel_registered_task(state: &AppState, task_id: &str) -> bool {
    state.cancel_task_handle(task_id)
}
