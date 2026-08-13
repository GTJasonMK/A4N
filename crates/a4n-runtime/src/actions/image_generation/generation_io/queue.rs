use crate::AppState;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use a4n_queue::queue::TaskWorkflowContext;
use std::future::Future;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run_queued_request_with_cancel<F, Fut, T>(
    state: &AppState,
    task_type: QueueTaskKind,
    priority: TaskPriority,
    project_id: Option<i64>,
    cancel_token: CancellationToken,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    run_queued_request_with_cancel_and_workflow(
        state,
        task_type,
        priority,
        project_id,
        None,
        cancel_token,
        operation,
    )
    .await
}

pub(crate) async fn run_queued_request_with_cancel_and_workflow<F, Fut, T>(
    state: &AppState,
    task_type: QueueTaskKind,
    priority: TaskPriority,
    project_id: Option<i64>,
    workflow_run_id: Option<i64>,
    cancel_token: CancellationToken,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let workflow = workflow_run_id
        .map(TaskWorkflowContext::new)
        .transpose()
        .map_err(|error| error.to_string())?;
    let running = state
        .orchestrator()
        .queue_start_task_with_cancel(task_type, priority, project_id, workflow, cancel_token)
        .await
        .map_err(|error| error.to_string())?;
    running.finish_message(operation().await).await
}
