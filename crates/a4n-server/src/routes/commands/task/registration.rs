use std::collections::VecDeque;
use std::sync::Arc;

use a4n_core::event::channel::{TaskHandle, TaskProgress, TaskType};
use a4n_runtime::host_api::task_records as task_records_actions;

use super::super::ServerState;
use super::events::emit_workflow_progress;

pub(in crate::routes::commands) async fn register_web_task(
    state: &ServerState,
    task_id: Option<String>,
    task_type: TaskType,
    project_id: impl Into<Option<i64>>,
    total: u32,
    message: &str,
) -> Option<Arc<TaskHandle>> {
    let task_id = task_id?;
    let handle = task_records_actions::register_task_handle(
        state.runtime(),
        TaskHandle {
            task_id: task_id.clone(),
            cancel_token: tokio_util::sync::CancellationToken::new(),
            progress: Arc::new(parking_lot::RwLock::new(TaskProgress::with_meta(
                0,
                total,
                message,
                Some(task_type.progress_meta(message)),
            ))),
            progress_events: Arc::new(parking_lot::RwLock::new(VecDeque::new())),
            task_type,
            project_id: project_id.into(),
            root_project_id: None,
            source_snapshot_id: None,
            task_scope: None,
        },
    );
    emit_workflow_progress(state, &task_id, 0, total, message).await;
    Some(handle)
}
