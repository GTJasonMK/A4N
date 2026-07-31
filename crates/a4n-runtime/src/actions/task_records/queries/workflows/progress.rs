use crate::{AppState, actions::app_error_message};

use super::super::super::{
    TaskProgressEventSummary, WorkflowProgressEventsInput,
    access::{
        verify_task_access, verify_task_handle_access, verify_workflow_run_access,
        verify_workflow_run_summary_access,
    },
    projections::task_progress_event_summary,
};

pub async fn get_workflow_progress_events(
    state: &AppState,
    user_id: i64,
    input: WorkflowProgressEventsInput,
) -> Result<Vec<TaskProgressEventSummary>, String> {
    let limit = input.limit.unwrap_or(50).clamp(1, 200);
    if let Some(workflow_run_id) = input.workflow_run_id {
        return workflow_progress_events_for_user(state, user_id, workflow_run_id, limit).await;
    }
    let Some(task_id) = input.task_id else {
        return Err("必须提供 task_id 或 workflow_run_id".to_string());
    };
    if let Some(handle) = state.task_handle(&task_id) {
        verify_task_handle_access(state, user_id, &handle).await?;
        return Ok(handle
            .progress_events
            .read()
            .iter()
            .cloned()
            .into_iter()
            .rev()
            .take(limit)
            .map(task_progress_event_summary)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect());
    }
    if let Some(task) = state
        .workflow()
        .persisted_task_by_id(&task_id)
        .await
        .map_err(app_error_message)?
    {
        verify_task_access(state, user_id, &task_id).await?;
        return match task.workflow_run_id {
            Some(workflow_run_id) => {
                workflow_progress_events_for_user(state, user_id, workflow_run_id, limit).await
            }
            None => Ok(Vec::new()),
        };
    }
    if let Some(run) = state
        .workflow()
        .workflow_run_model_by_active_task_id(&task_id)
        .await
        .map_err(app_error_message)?
    {
        verify_workflow_run_access(state, user_id, &run).await?;
        return state
            .workflow()
            .workflow_progress_events(run.id, limit)
            .await
            .map_err(app_error_message);
    }
    Err(format!("任务不存在: {task_id}"))
}

async fn workflow_progress_events_for_user(
    state: &AppState,
    user_id: i64,
    workflow_run_id: i64,
    limit: usize,
) -> Result<Vec<TaskProgressEventSummary>, String> {
    let run = state
        .workflow()
        .workflow_run_summary(workflow_run_id)
        .await
        .map_err(app_error_message)?;
    verify_workflow_run_summary_access(state, user_id, &run).await?;
    state
        .workflow()
        .workflow_progress_events(workflow_run_id, limit)
        .await
        .map_err(app_error_message)
}
