use crate::AppState;
use a4n_queue::TaskWorkflowContext;
use a4n_queue::priority::{TaskKind as QueueTaskKind, TaskPriority};
use std::future::Future;
use tokio_util::sync::CancellationToken;

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
    let result = match workflow {
        Some(workflow) => {
            state
                .provider_runtime()
                .queue_run_tracked_with_workflow_context_and_cancel(
                    task_type,
                    priority,
                    project_id,
                    Some(workflow),
                    cancel_token,
                    || async {
                        operation()
                            .await
                            .map_err(|error| a4n_error::AppError::Internal(anyhow::anyhow!(error)))
                    },
                )
                .await
        }
        None => {
            state
                .provider_runtime()
                .queue_run_tracked_with_cancel(
                    task_type,
                    priority,
                    project_id,
                    cancel_token,
                    || async {
                        operation()
                            .await
                            .map_err(|error| a4n_error::AppError::Internal(anyhow::anyhow!(error)))
                    },
                )
                .await
        }
    };
    result.map_err(|error| error.to_string())
}
