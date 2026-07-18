use a4n_queue::priority::TaskInfo;

use super::super::super::{WorkflowProgressSummary, WorkflowStateSummary};
use super::events::queue_progress_event_summary;

pub(in crate::actions::task_records) fn task_info_to_workflow(
    info: &TaskInfo,
) -> WorkflowStateSummary {
    WorkflowStateSummary {
        task_id: info.id.clone(),
        status: "running".to_string(),
        project_id: info.project_id,
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        stage: info
            .progress
            .as_ref()
            .map(|progress| progress.message.clone()),
        progress: info
            .progress
            .as_ref()
            .map(|progress| WorkflowProgressSummary {
                task_id: info.id.clone(),
                stage: progress
                    .meta
                    .as_ref()
                    .map(|meta| meta.stage_label.clone())
                    .unwrap_or_else(|| progress.message.clone()),
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                percent: progress.percent() * 100.0,
                meta: progress.meta.clone(),
                updated_at: progress.updated_at.clone(),
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
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: None,
    }
}
