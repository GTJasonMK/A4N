use a4n_core::event::channel::{TaskHandle, TaskKind, TaskProgress};
use a4n_core::event::progress::ProgressMeta;
use a4n_runtime::AppState;
use a4n_runtime::host_api::task_records;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;

pub(crate) struct WorkflowProgressTask {
    task_handle: Option<Arc<TaskHandle>>,
    task_type: TaskKind,
}

pub(crate) struct WorkflowTaskContext {
    pub project_id: Option<i64>,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub task_scope: Option<String>,
}

impl WorkflowProgressTask {
    pub(crate) fn new(
        state: &AppState,
        owner_user_id: Option<i64>,
        task_id: Option<String>,
        task_type: TaskKind,
        project_id: Option<i64>,
        total: u32,
        message: impl Into<String>,
    ) -> Self {
        Self::new_with_context(
            state,
            owner_user_id,
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
        state: &AppState,
        owner_user_id: Option<i64>,
        task_id: Option<String>,
        task_type: TaskKind,
        context: WorkflowTaskContext,
        total: u32,
        message: impl Into<String>,
    ) -> Self {
        let total = total.max(1);
        let message = message.into();
        let task_handle = task_id.map(|task_id| {
            let cancel_token = CancellationToken::new();
            task_records::register_task_handle(
                state,
                TaskHandle {
                    task_id,
                    owner_user_id,
                    cancel_token,
                    progress: Arc::new(RwLock::new(TaskProgress::with_meta(
                        0,
                        total,
                        message.clone(),
                        Some(task_type.progress_meta(&message)),
                    ))),
                    progress_events: Arc::new(RwLock::new(VecDeque::new())),
                    task_type,
                    project_id: context.project_id,
                    root_project_id: context.root_project_id,
                    source_snapshot_id: context.source_snapshot_id,
                    task_scope: context.task_scope,
                },
            )
        });

        Self {
            task_handle,
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
            handle.update_progress_with_meta(current, total, message, meta);
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
            handle.update_progress_bytes(current, total, progress_total, message);
        }
    }

    /// 转发模型输出的活体信号（流式尾巴）。只更新快照，不推送进度事件。
    pub(crate) fn stream_text(&self, text: &str) {
        if let Some(handle) = self.task_handle.as_ref() {
            handle.update_stream_text(text);
        }
    }

    pub(crate) fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self
            .task_handle
            .as_ref()
            .is_some_and(|handle| handle.is_cancelled())
        {
            return Err(message.to_string());
        }
        Ok(())
    }

    pub(crate) fn cancel_token(&self) -> CancellationToken {
        self.task_handle
            .as_ref()
            .map(|handle| handle.cancel_token.clone())
            .unwrap_or_default()
    }

    pub(crate) fn task_id(&self) -> Option<&str> {
        self.task_handle
            .as_ref()
            .map(|handle| handle.task_id.as_str())
    }

    pub(crate) fn remove(&self, state: &AppState) {
        if let Some(handle) = self.task_handle.as_ref() {
            task_records::remove_task_handle(state, &handle.task_id);
        }
    }
}
