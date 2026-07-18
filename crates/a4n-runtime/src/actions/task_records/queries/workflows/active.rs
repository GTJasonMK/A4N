use crate::{AppState, actions::app_error_message};
use a4n_queue::priority::TaskStatus;
use std::collections::HashSet;

use super::super::super::{
    WorkflowStateSummary,
    access::workflow_visible_to_user,
    projections::{task_info_to_workflow, task_manager_snapshot_to_workflow},
};

pub async fn list_active_workflows(state: &AppState) -> Result<Vec<WorkflowStateSummary>, String> {
    let tasks = state
        .orchestrator
        .persisted_task_history(None, 0, 100)
        .await
        .map_err(app_error_message)?;
    let mut active = tasks
        .iter()
        .filter(|task| matches!(task.task.status, TaskStatus::Running))
        .map(|task| task_info_to_workflow(&task.task))
        .collect::<Vec<_>>();
    let mut seen = active
        .iter()
        .map(|item| item.task_id.clone())
        .collect::<HashSet<_>>();
    for snapshot in state.task_manager.list_active_snapshots() {
        if seen.insert(snapshot.task_id.clone()) {
            active.push(task_manager_snapshot_to_workflow(snapshot));
        }
    }
    Ok(active)
}

pub async fn list_active_workflows_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<WorkflowStateSummary>, String> {
    let active = list_active_workflows(state).await?;
    let mut visible = Vec::new();
    for workflow in active {
        if workflow_visible_to_user(state, user_id, &workflow).await? {
            visible.push(workflow);
        }
    }
    Ok(visible)
}
