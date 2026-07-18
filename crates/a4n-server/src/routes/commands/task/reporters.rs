use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::event::channel::TaskHandle;

use super::super::ServerState;
use super::events::{emit_workflow_chunk, emit_workflow_progress_with_meta};

pub(in crate::routes::commands) fn web_progress_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync + 'static {
    let state = state.clone();
    move |current, total, message| {
        let meta = task
            .as_ref()
            .map(|handle| handle.task_type.progress_meta(message));
        if let Some(handle) = task.as_ref() {
            if handle.is_cancelled() {
                return Err(AppError::Cancelled);
            }
            handle.update_progress_with_meta(current, total, message, meta.clone());
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            let message = message.to_string();
            tokio::spawn(async move {
                emit_workflow_progress_with_meta(&state, &task_id, current, total, &message, meta)
                    .await;
            });
        }
        Ok(())
    }
}

pub(in crate::routes::commands) fn web_chunk_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl FnMut(&str, u32) -> Result<(), AppError> + Send + 'static {
    let state = state.clone();
    move |delta, index| {
        if task.as_ref().is_some_and(|handle| handle.is_cancelled()) {
            return Err(AppError::Cancelled);
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            let content = delta.to_string();
            tokio::spawn(async move {
                emit_workflow_chunk(&state, &task_id, &content, index).await;
            });
        }
        Ok(())
    }
}

pub(in crate::routes::commands) fn web_fire_and_forget_progress_reporter(
    state: &ServerState,
    task: Option<Arc<TaskHandle>>,
    task_id: Option<String>,
) -> impl Fn(u32, u32, &str) + Send + Sync + 'static {
    let state = state.clone();
    move |current, total, message| {
        let meta = task
            .as_ref()
            .map(|handle| handle.task_type.progress_meta(message));
        if let Some(handle) = task.as_ref() {
            handle.update_progress_with_meta(current, total, message, meta.clone());
        }
        if let Some(task_id) = task_id.as_ref() {
            let state = state.clone();
            let task_id = task_id.clone();
            let message = message.to_string();
            tokio::spawn(async move {
                emit_workflow_progress_with_meta(&state, &task_id, current, total, &message, meta)
                    .await;
            });
        }
    }
}
