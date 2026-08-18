use crate::{AppState, actions::app_error_message};
use a4n_api_contract::workflow::WorkflowStateSummary;
use a4n_queue::priority::TaskStatus;
use std::collections::HashSet;

use super::super::super::{
    access::task_scope_visible_to_user,
    projections::{task_info_to_workflow, task_manager_snapshot_to_workflow},
};

pub async fn list_active_workflows_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<WorkflowStateSummary>, String> {
    let tasks = state
        .orchestrator()
        .persisted_task_history(None, 0, 100)
        .await
        .map_err(app_error_message)?;
    let mut active = Vec::new();
    let mut seen = HashSet::new();
    for persisted in tasks {
        let task = persisted.task;
        if !matches!(task.status, TaskStatus::Running)
            || !task_scope_visible_to_user(state, user_id, task.project_id, task.root_project_id)
                .await?
        {
            continue;
        }
        seen.insert(task.id.clone());
        active.push(task_info_to_workflow(&task));
    }
    for snapshot in state.orchestrator().active_task_snapshots() {
        if seen.contains(&snapshot.task_id)
            || !task_scope_visible_to_user(
                state,
                user_id,
                snapshot.project_id,
                snapshot.root_project_id,
            )
            .await?
        {
            continue;
        }
        seen.insert(snapshot.task_id.clone());
        active.push(task_manager_snapshot_to_workflow(snapshot));
    }
    Ok(active)
}
