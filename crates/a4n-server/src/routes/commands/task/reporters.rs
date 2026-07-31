use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::event::channel::TaskHandle;

pub(in crate::routes::commands) fn web_progress_reporter(
    task: Option<Arc<TaskHandle>>,
) -> impl Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'static {
    move |current, total, message| {
        if let Some(handle) = task.as_ref() {
            if handle.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            handle.update_progress(current, total, message);
        }
        Ok(())
    }
}

pub(in crate::routes::commands) fn web_fire_and_forget_progress_reporter(
    task: Option<Arc<TaskHandle>>,
) -> impl Fn(u32, u32, &str) + Send + Sync + 'static {
    move |current, total, message| {
        if let Some(handle) = task.as_ref() {
            handle.update_progress(current, total, message);
        }
    }
}
