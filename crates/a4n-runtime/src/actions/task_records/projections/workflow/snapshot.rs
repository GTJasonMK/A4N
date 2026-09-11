use a4n_api_contract::{task::TaskProgressSummary, workflow::WorkflowStateSummary};
use a4n_task_runtime::channel::TaskSnapshot;

use super::events::task_progress_event;

pub(in crate::actions::task_records) fn task_manager_snapshot_to_workflow(
    snapshot: TaskSnapshot,
) -> WorkflowStateSummary {
    let task_id = snapshot.task_id;
    WorkflowStateSummary {
        task_id: task_id.clone(),
        root_project_id: snapshot.root_project_id,
        source_snapshot_id: snapshot.source_snapshot_id,
        task_scope: snapshot.task_scope,
        progress: Some(TaskProgressSummary {
            current: snapshot.progress.current,
            total: snapshot.progress.total,
            message: snapshot.progress.message,
            meta: snapshot.progress.meta,
        }),
        progress_events: snapshot
            .progress_events
            .into_iter()
            .map(task_progress_event)
            .collect(),
    }
}
