use std::collections::{BTreeMap, BTreeSet};

use crate::{AppState, actions::app_error_message};
use a4n_orchestrator::workflow::cancel_workflow_requires_target;

use super::super::{
    CancelWorkflowInput, WorkflowRunSummary,
    access::{verify_task_access, verify_task_handle_access, verify_workflow_run_summary_access},
};

pub async fn cancel_workflow(
    state: &AppState,
    user_id: i64,
    input: CancelWorkflowInput,
) -> Result<(), String> {
    let task_id = input
        .task_id
        .as_deref()
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty());
    cancel_workflow_requires_target(task_id.is_some(), input.workflow_run_id.is_some())?;

    let mut workflows = BTreeMap::new();
    if let Some(workflow_run_id) = input.workflow_run_id {
        let run = authorized_workflow_run(state, user_id, workflow_run_id).await?;
        workflows.insert(run.id, run);
    }

    let mut persisted_task = None;
    let mut task_handle = None;
    let mut task_workflow_ids = BTreeSet::new();
    if let Some(task_id) = task_id {
        persisted_task = state
            .orchestrator()
            .persisted_task_by_id(task_id)
            .await
            .map_err(app_error_message)?;
        task_handle = state.task_handle(task_id);

        if let Some(task) = persisted_task.as_ref() {
            verify_task_access(state, user_id, task_id).await?;
            if let Some(workflow_run_id) = task.workflow_run_id {
                let run = authorized_workflow_run(state, user_id, workflow_run_id).await?;
                task_workflow_ids.insert(run.id);
                workflows.insert(run.id, run);
            }
        }
        if let Some(handle) = task_handle.as_ref() {
            verify_task_handle_access(state, user_id, handle).await?;
        }
        if let Some(run) = state
            .orchestrator()
            .workflow_run_model_by_active_task_id(task_id)
            .await
            .map_err(app_error_message)?
        {
            let run = authorized_workflow_run(state, user_id, run.id).await?;
            task_workflow_ids.insert(run.id);
            workflows.insert(run.id, run);
        }

        if persisted_task.is_none() && task_handle.is_none() && task_workflow_ids.is_empty() {
            state.cancel_or_remember_task_handle(task_id, user_id);
        }
    }

    let mut cancelled_workflow_ids = BTreeSet::new();
    for run in workflows.values() {
        if cancel_authorized_workflow_run(state, run).await? {
            cancelled_workflow_ids.insert(run.id);
        }
    }

    if let Some(task_id) = task_id {
        let persisted_task_is_terminal = persisted_task.as_ref().is_some_and(|task| {
            matches!(
                task.task.status.as_str(),
                "completed" | "failed" | "cancelled" | "interrupted"
            )
        });
        let has_linked_workflow = !task_workflow_ids.is_empty();
        let cancelled_linked_workflow = task_workflow_ids
            .iter()
            .any(|workflow_run_id| cancelled_workflow_ids.contains(workflow_run_id));
        let should_cancel_task = task_cancel_signal_allowed(
            persisted_task_is_terminal,
            has_linked_workflow,
            cancelled_linked_workflow,
        );
        if should_cancel_task && task_handle.is_some() {
            state.cancel_task_handle(task_id);
        }
        if should_cancel_task
            && persisted_task.is_some()
            && let Err(error) = state.orchestrator().queue_cancel(task_id).await
        {
            if task_workflow_ids.is_empty() {
                return Err(error.to_string());
            }
            tracing::warn!(
                task_id,
                error = %error,
                "workflow 已持久化取消，但关联队列任务未能同步取消"
            );
        }
    }
    Ok(())
}

async fn authorized_workflow_run(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
) -> Result<WorkflowRunSummary, String> {
    let run = state
        .orchestrator()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    Ok(run)
}

async fn cancel_authorized_workflow_run(
    state: &AppState,
    run: &WorkflowRunSummary,
) -> Result<bool, String> {
    let cancellation = state
        .orchestrator()
        .cancel_workflow_run_and_interrupt_videos(run.id, "用户取消画布运行")
        .await
        .map_err(app_error_message)?;
    if cancellation.already_terminal {
        return Ok(false);
    }
    Ok(true)
}

fn task_cancel_signal_allowed(
    persisted_task_is_terminal: bool,
    has_linked_workflow: bool,
    cancelled_linked_workflow: bool,
) -> bool {
    if has_linked_workflow {
        cancelled_linked_workflow
    } else {
        !persisted_task_is_terminal
    }
}

#[cfg(test)]
mod tests {
    use super::task_cancel_signal_allowed;

    #[test]
    fn terminal_authority_blocks_late_cancel_signals() {
        assert!(!task_cancel_signal_allowed(true, false, false));
        assert!(!task_cancel_signal_allowed(false, true, false));
        assert!(task_cancel_signal_allowed(false, false, false));
        assert!(task_cancel_signal_allowed(false, true, true));
    }
}
