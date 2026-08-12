use a4n_api_contract::{task::TaskProgressSummary, workflow::WorkflowStateSummary};
use a4n_queue::priority::TaskInfo;

use super::events::queue_progress_event_summary;

pub(in crate::actions::task_records) fn task_info_to_workflow(
    info: &TaskInfo,
) -> WorkflowStateSummary {
    WorkflowStateSummary {
        task_id: info.id.clone(),
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| TaskProgressSummary {
            current: progress.current,
            total: progress.total,
            message: progress.message.clone(),
            meta: progress.meta.clone(),
        }),
        progress_events: info
            .progress
            .as_ref()
            .map(|progress| {
                queue_progress_event_summary(
                    &info.id,
                    progress,
                    info.started_at.unwrap_or(info.created_at),
                )
            })
            .into_iter()
            .collect(),
    }
}
