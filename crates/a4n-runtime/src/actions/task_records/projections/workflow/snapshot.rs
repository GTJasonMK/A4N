use a4n_core::event::channel::TaskSnapshot;

use super::super::super::{WorkflowProgressSummary, WorkflowStateSummary};
use super::events::task_progress_event_summary;

pub(in crate::actions::task_records) fn task_manager_snapshot_to_workflow(
    snapshot: TaskSnapshot,
) -> WorkflowStateSummary {
    let task_id = snapshot.task_id;
    let stage = snapshot.progress.message.clone();
    let percent = snapshot.progress.percent() * 100.0;
    WorkflowStateSummary {
        task_id: task_id.clone(),
        status: "running".to_string(),
        project_id: snapshot.project_id,
        root_project_id: snapshot.root_project_id,
        source_snapshot_id: snapshot.source_snapshot_id,
        task_scope: snapshot.task_scope,
        stage: Some(snapshot.progress.message.clone()),
        progress: Some(WorkflowProgressSummary {
            task_id: task_id.clone(),
            stage,
            current: snapshot.progress.current,
            total: snapshot.progress.total,
            message: snapshot.progress.message,
            percent,
            meta: snapshot.progress.meta,
            updated_at: snapshot.progress.updated_at,
        }),
        progress_events: snapshot
            .progress_events
            .into_iter()
            .map(|event| task_progress_event_summary(&task_id, event))
            .collect(),
        started_at: None,
        completed_at: None,
    }
}
