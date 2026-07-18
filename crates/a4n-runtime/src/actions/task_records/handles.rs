use crate::AppState;
use a4n_core::event::channel::TaskHandle;
use std::sync::Arc;

pub fn register_task_handle(state: &AppState, handle: TaskHandle) -> Arc<TaskHandle> {
    state.task_manager.register(handle)
}

pub fn remove_task_handle(state: &AppState, task_id: &str) -> Option<Arc<TaskHandle>> {
    state.task_manager.remove(task_id)
}

pub fn cancel_registered_task(state: &AppState, task_id: &str) -> bool {
    state.task_manager.cancel(task_id)
}
