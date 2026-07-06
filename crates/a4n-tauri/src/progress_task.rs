use crate::events::bridge::EventBridge;
use a4n_core::event::channel::{TaskHandle, TaskProgress, TaskType};
use a4n_core::event::sse::ProgressMeta;
use a4n_runtime::AppState;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tauri::Window;
use tokio_util::sync::CancellationToken;

pub(crate) struct WorkflowProgressTask {
    task_id: Option<String>,
    bridge: Option<EventBridge>,
    task_handle: Option<Arc<TaskHandle>>,
    cancel_token: Option<CancellationToken>,
    task_type: TaskType,
}

pub(crate) struct WorkflowTaskContext {
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
}

impl WorkflowProgressTask {
    pub(crate) fn new(
        window: Window,
        state: &AppState,
        task_id: Option<String>,
        task_type: TaskType,
        project_id: Option<i64>,
        total: u32,
        message: impl Into<String>,
    ) -> Self {
        Self::new_with_context(
            window,
            state,
            task_id,
            task_type,
            WorkflowTaskContext {
                project_id,
                root_project_id: None,
                source_snapshot_id: None,
                task_scope: None,
            },
            total,
            message,
        )
    }

    pub(crate) fn new_with_context(
        window: Window,
        state: &AppState,
        task_id: Option<String>,
        task_type: TaskType,
        context: WorkflowTaskContext,
        total: u32,
        message: impl Into<String>,
    ) -> Self {
        let total = total.max(1);
        let message = message.into();
        let bridge = task_id.as_ref().map(|_| EventBridge::new(window));
        let cancel_token = task_id.as_ref().map(|_| CancellationToken::new());
        let task_handle = match (task_id.as_ref(), cancel_token.as_ref()) {
            (Some(task_id), Some(cancel_token)) => Some(state.task_manager.register(TaskHandle {
                task_id: task_id.clone(),
                cancel_token: cancel_token.clone(),
                progress: Arc::new(RwLock::new(TaskProgress::with_meta(
                    0,
                    total,
                    message.clone(),
                    Some(task_type.progress_meta(&message)),
                ))),
                progress_events: Arc::new(RwLock::new(VecDeque::new())),
                task_type: task_type.clone(),
                project_id: context.project_id,
                root_project_id: context.root_project_id,
                source_snapshot_id: context.source_snapshot_id,
                task_scope: context.task_scope,
            })),
            _ => None,
        };

        Self {
            task_id,
            bridge,
            task_handle,
            cancel_token,
            task_type,
        }
    }

    pub(crate) fn progress(&self, current: u32, total: u32, message: &str) {
        self.progress_with_meta(current, total, message, None);
    }

    pub(crate) fn progress_with_meta(
        &self,
        current: u32,
        total: u32,
        message: &str,
        meta: Option<ProgressMeta>,
    ) {
        let total = total.max(1);
        let meta = meta.or_else(|| Some(self.task_type.progress_meta(message)));
        if let Some(handle) = self.task_handle.as_ref() {
            handle.update_progress_with_meta(current, total, message, meta.clone());
        }
        if let (Some(task_id), Some(bridge)) = (self.task_id.as_ref(), self.bridge.as_ref()) {
            let _ = bridge.emit_progress_with_meta(task_id, current, total, message, meta);
        }
    }

    pub(crate) fn progress_bytes(
        &self,
        current: u64,
        total: u64,
        progress_total: u32,
        message: &str,
    ) {
        if let Some(handle) = self.task_handle.as_ref() {
            let scaled_current = (current.min(total) * u64::from(progress_total))
                .checked_div(total)
                .unwrap_or(0)
                .min(u64::from(progress_total)) as u32;
            handle.update_progress(scaled_current, progress_total, message);
        }
        if let (Some(task_id), Some(bridge)) = (self.task_id.as_ref(), self.bridge.as_ref()) {
            let _ = bridge.emit_progress_bytes(task_id, current, total, message);
        }
    }

    pub(crate) fn chunk(&self, content: &str, index: u32) {
        if let (Some(task_id), Some(bridge)) = (self.task_id.as_ref(), self.bridge.as_ref()) {
            let _ = bridge.emit_chunk(task_id, content, index);
        }
    }

    pub(crate) fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self
            .cancel_token
            .as_ref()
            .is_some_and(|token| token.is_cancelled())
        {
            return Err(message.to_string());
        }
        Ok(())
    }

    pub(crate) fn cancel_token(&self) -> CancellationToken {
        self.cancel_token.clone().unwrap_or_default()
    }

    pub(crate) fn complete(&self, summary: impl Into<String>) {
        if let (Some(task_id), Some(bridge)) = (self.task_id.as_ref(), self.bridge.as_ref()) {
            let _ = bridge.emit_complete(task_id, Some(summary.into()));
        }
    }

    pub(crate) fn error(&self, code: &str, message: &str) {
        if let (Some(task_id), Some(bridge)) = (self.task_id.as_ref(), self.bridge.as_ref()) {
            let error_code = if message.contains("已停止") || message.contains("已取消") {
                "cancelled"
            } else {
                code
            };
            let _ = bridge.emit_error(task_id, error_code, message, false);
        }
    }

    pub(crate) fn finish_result<T, F, S>(
        self,
        state: &AppState,
        result: &Result<T, String>,
        error_code: &str,
        success_summary: F,
    ) where
        F: FnOnce(&T) -> S,
        S: Into<String>,
    {
        match result {
            Ok(value) => self.complete(success_summary(value)),
            Err(err) => self.error(error_code, err),
        }
        self.remove(state);
    }

    pub(crate) fn remove(&self, state: &AppState) {
        if let Some(task_id) = self.task_id.as_ref() {
            state.task_manager.remove(task_id);
        }
    }
}
