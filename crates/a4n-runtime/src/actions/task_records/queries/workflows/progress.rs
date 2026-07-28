use crate::{AppState, actions::app_error_message};

use super::super::super::{
    TaskProgressEventSummary, WorkflowProgressEventsInput,
    access::{verify_task_access, verify_workflow_run_summary_access},
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
    verify_task_access(state, user_id, &task_id).await?;
    if let Some(snapshot) = state
        .active_task_snapshots()
        .into_iter()
        .find(|snapshot| snapshot.task_id == task_id)
    {
        return Ok(snapshot
            .progress_events
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
        && let Some(workflow_run_id) = task.workflow_run_id
    {
        return workflow_progress_events_for_user(state, user_id, workflow_run_id, limit).await;
    }
    Ok(Vec::new())
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
