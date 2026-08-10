use std::collections::VecDeque;
use std::sync::Arc;

use a4n_core::event::channel::{TaskHandle, TaskKind, TaskProgress};
use a4n_runtime::host_api::task_records as task_records_actions;

use super::super::ServerState;
pub(in crate::routes::commands) fn register_web_task(
    state: &ServerState,
    owner_user_id: i64,
    task_id: Option<String>,
    task_type: TaskKind,
    project_id: impl Into<Option<i64>>,
    total: u32,
    message: &str,
) -> Option<Arc<TaskHandle>> {
    let task_id = task_id?;
    let handle = task_records_actions::register_task_handle(
        state.runtime(),
        TaskHandle {
            task_id,
            owner_user_id: Some(owner_user_id),
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
    Some(handle)
}
