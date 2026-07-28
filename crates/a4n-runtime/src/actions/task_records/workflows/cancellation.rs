use crate::{AppState, actions::app_error_message};
use a4n_orchestrator::workflow::cancel_workflow_requires_target;

use super::super::{
    CancelWorkflowInput,
    access::{verify_task_access, verify_workflow_run_summary_access},
};

pub async fn cancel_workflow(
    state: &AppState,
    user_id: i64,
    input: CancelWorkflowInput,
) -> Result<(), String> {
    let has_task_id = input
        .task_id
        .as_deref()
        .is_some_and(|task_id| !task_id.trim().is_empty());
    cancel_workflow_requires_target(has_task_id, input.workflow_run_id.is_some())?;
    if let Some(task_id) = input
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty())
    {
        verify_task_access(state, user_id, task_id).await?;
        if state.cancel_task_handle(task_id) {
            return Ok(());
        }
        state
            .task_queue()
            .queue_cancel(task_id)
            .await
            .map_err(|err| err.to_string())?;
    }
    if let Some(workflow_run_id) = input.workflow_run_id {
        cancel_workflow_run_for_user(state, user_id, workflow_run_id).await?;
    }
    Ok(())
}

async fn cancel_workflow_run_for_user(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<(), String> {
    let run = state
        .workflow()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    if let Some(task_id) = run.active_task_id.as_deref() {
        let _ = state.cancel_task_handle(task_id);
        state
            .task_queue()
            .queue_cancel(task_id)
            .await
            .map_err(|error| error.to_string())?;
    }
    let cancellation = state
        .workflow()
        .cancel_workflow_run_and_interrupt_videos(workflow_run_id, "用户取消画布运行")
        .await;
    cancellation.map_err(app_error_message)
}
