use crate::AppState;
use a4n_queue::priority::{TaskPriority, TaskQueue, TaskType as QueueTaskType};
use a4n_queue::queue::TaskWorkflowContext;
use std::future::Future;
use tokio_util::sync::CancellationToken;

pub(crate) async fn run_queued_request_with_cancel<F, Fut, T>(
    state: &AppState,
    task_type: QueueTaskType,
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
    task_type: QueueTaskType,
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
    let submitted_id = if let Some(workflow_run_id) = workflow_run_id {
        state
            .task_queue
            .submit_with_workflow_context(
                task_type,
                priority,
                project_id,
                TaskWorkflowContext::new(workflow_run_id).map_err(|err| err.to_string())?,
            )
            .await
            .map_err(|err| err.to_string())?
    } else {
        state
            .task_queue
            .submit(task_type, priority, project_id)
            .await
            .map_err(|err| err.to_string())?
    };
    let mut submitted_guard = state.task_queue.submitted_task_guard(submitted_id.clone());
    let Some(permit) = state
        .task_queue
        .acquire_task_with_cancel(&submitted_id, cancel_token)
        .await
        .map_err(|err| err.to_string())?
    else {
        let cancel_result = state.task_queue.cancel(&submitted_id).await;
        submitted_guard.disarm();
        return match cancel_result {
            Ok(()) => Err("任务已取消".to_string()),
            Err(status_error) => Err(format!(
                "任务已取消；同步队列任务取消状态失败: {status_error}"
            )),
        };
    };
    submitted_guard.disarm();
    let running_id = permit.task_id().to_string();

    match operation().await {
        Ok(value) => {
            permit.complete();
            state
                .task_queue
                .mark_done(&running_id)
                .await
                .map_err(|error| format!("队列任务业务操作已完成，但同步完成状态失败: {error}"))?;
            Ok(value)
        }
        Err(mut error) => {
            permit.fail(error.clone());
            if let Err(status_error) = state.task_queue.mark_failed(&running_id, &error).await {
                error.push_str(&format!("；同步队列任务失败状态失败: {status_error}"));
            }
            Err(error)
        }
    }
}
