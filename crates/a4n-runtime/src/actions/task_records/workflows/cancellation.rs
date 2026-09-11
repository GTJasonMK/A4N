use std::collections::BTreeMap;

use crate::{AppState, actions::app_error_message};
use a4n_api_contract::workflow::{CancelWorkflowInput, WorkflowRunSummary};
use a4n_domain::workflow::{cancel_workflow_requires_target, require_positive_workflow_run_id};
use a4n_task_runtime::channel::normalize_task_id;

use super::super::access::{
    verify_task_access, verify_task_handle_access, verify_workflow_run_summary_access,
};

const USER_CANCELLED_WORKFLOW_REASON: &str = "用户取消工作流";

pub async fn cancel_workflow(
    state: &AppState,
    user_id: i64,
    input: CancelWorkflowInput,
) -> Result<(), String> {
    let task_id = input
        .task_id
        .as_deref()
        .map(normalize_task_id)
        .transpose()
        .map_err(|error| error.to_string())?;
    let task_id = task_id.as_deref();
    cancel_workflow_requires_target(task_id.is_some(), input.workflow_run_id.is_some())?;
    if let Some(workflow_run_id) = input.workflow_run_id {
        require_positive_workflow_run_id(workflow_run_id)?;
    }

    let mut workflows = BTreeMap::new();
    if let Some(workflow_run_id) = input.workflow_run_id {
        let run = authorized_workflow_run(state, user_id, workflow_run_id).await?;
        workflows.insert(run.id, run);
    }

    let mut persisted_task = None;
    let mut task_handle = None;
    let mut task_has_linked_workflow = false;
    if let Some(task_id) = task_id {
        persisted_task = state
            .workflow_task_lookup_application()
            .task_by_id(task_id)
            .await
            .map_err(app_error_message)?;
        task_handle = state.task_manager().get(task_id);

        if let Some(task) = persisted_task.as_ref() {
            verify_task_access(state, user_id, task_id).await?;
            if let Some(workflow_run_id) = task.workflow_run_id {
                let run = authorized_workflow_run(state, user_id, workflow_run_id).await?;
                task_has_linked_workflow = true;
                workflows.insert(run.id, run);
            }
        }
        if let Some(handle) = task_handle.as_ref() {
            verify_task_handle_access(state, user_id, handle).await?;
        }
        if let Some(run) = state
            .workflow_task_lookup_application()
            .workflow_access_by_active_task_id(task_id)
            .await
            .map_err(app_error_message)?
        {
            let run = authorized_workflow_run(state, user_id, run.workflow_run_id).await?;
            task_has_linked_workflow = true;
            workflows.insert(run.id, run);
        }

        if persisted_task.is_none() && task_handle.is_none() && !task_has_linked_workflow {
            state.task_manager().cancel_or_remember(task_id, user_id);
        }
    }

    for run in workflows.values() {
        state
            .workflow_cancellation_application()
            .cancel(run.id, USER_CANCELLED_WORKFLOW_REASON)
            .await
            .map_err(app_error_message)?;
    }

    if let Some(task_id) = task_id
        && !task_has_linked_workflow
        && !persisted_task.as_ref().is_some_and(|task| {
            matches!(
                task.status.as_str(),
                "completed" | "failed" | "cancelled" | "interrupted"
            )
        })
    {
        if task_handle.is_some() {
            state.task_manager().cancel(task_id);
        }
        if persisted_task.is_some() {
            state
                .task_cancellation_application()
                .cancel(task_id)
                .await
                .map_err(app_error_message)?;
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
        .workflow_task_lookup_application()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    Ok(run)
}
