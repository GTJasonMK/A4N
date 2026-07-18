use crate::state::AppState;
use a4n_core::error::AppError;
use a4n_queue::priority::{TaskPriority, TaskType as QueueTaskType};
use a4n_queue::queue::TaskWorkflowContext;
use std::future::Future;

pub(in crate::actions::canvas_workflow) async fn run_canvas_assembly_task<F, Fut, T>(
    state: &AppState,
    project_id: i64,
    workflow_run_id: i64,
    operation: F,
) -> Result<T, String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, String>>,
{
    let workflow_context =
        TaskWorkflowContext::new(workflow_run_id).map_err(|err| err.to_string())?;
    state
        .task_queue
        .run_tracked_with_workflow_context(
            QueueTaskType::Assembly,
            TaskPriority::Medium,
            Some(project_id),
            workflow_context,
            || async {
                operation()
                    .await
                    .map_err(|err| AppError::Internal(anyhow::anyhow!(err)))
            },
        )
        .await
        .map_err(|err| err.to_string())
}
