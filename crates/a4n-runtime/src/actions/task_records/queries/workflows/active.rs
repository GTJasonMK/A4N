use crate::{AppState, actions::admin_records};
use a4n_api_contract::workflow::WorkflowStateSummary;
use a4n_queue::priority::TaskStatus;
use std::collections::HashSet;

use super::super::super::{
    access::project_ids_visible_to_user, projections::task_manager_snapshot_to_workflow,
};

pub async fn list_active_workflows_for_user(
    state: &AppState,
    user_id: i64,
) -> Result<Vec<WorkflowStateSummary>, String> {
    let tasks = state
        .workflow_task_lookup_application()
        .active_tasks()
        .await
        .map_err(|error| error.to_string())?;
    let snapshots = state.task_manager().list_active_snapshots();
    let project_ids = tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Running.as_str())
        .filter_map(|task| task.project_id.or(task.root_project_id))
        .chain(
            snapshots
                .iter()
                .filter_map(|task| task.project_id.or(task.root_project_id)),
        )
        .collect::<Vec<_>>();
    let visible_project_ids = project_ids_visible_to_user(state, user_id, &project_ids).await?;
    let has_unscoped_task = tasks.iter().any(|persisted| {
        persisted.status == TaskStatus::Running.as_str()
            && persisted.project_id.is_none()
            && persisted.root_project_id.is_none()
    }) || snapshots
        .iter()
        .any(|task| task.project_id.is_none() && task.root_project_id.is_none());
    let unscoped_visible = if has_unscoped_task {
        admin_records::is_active_admin_user(state, user_id)
            .await
            .map_err(|err| err.to_string())?
    } else {
        false
    };
    let scope_visible = |project_id: Option<i64>, root_project_id: Option<i64>| {
        project_id
            .or(root_project_id)
            .map_or(unscoped_visible, |project_id| {
                visible_project_ids.contains(&project_id)
            })
    };
    let mut active = Vec::new();
    let mut seen = HashSet::new();
    for task in tasks {
        if task.status != TaskStatus::Running.as_str()
            || !scope_visible(task.project_id, task.root_project_id)
        {
            continue;
        }
        seen.insert(task.task_id.clone());
        active.push(task.workflow);
    }
    for snapshot in snapshots {
        if seen.contains(&snapshot.task_id)
            || !scope_visible(snapshot.project_id, snapshot.root_project_id)
        {
            continue;
        }
        seen.insert(snapshot.task_id.clone());
        active.push(task_manager_snapshot_to_workflow(snapshot));
    }
    Ok(active)
}
