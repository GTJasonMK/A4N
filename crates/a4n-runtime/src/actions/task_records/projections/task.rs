use a4n_queue::priority::TaskInfo;

use super::super::{TaskInfoSummary, TaskProgressSummary};
use super::workflow::queue_progress_event_summary;

pub(in crate::actions::task_records) fn task_info_summary(info: &TaskInfo) -> TaskInfoSummary {
    TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        project_id: info.project_id,
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| TaskProgressSummary {
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
            .map(|progress| queue_progress_event_summary(&info.id, progress, info.created_at))
            .into_iter()
            .collect(),
        created_at: Some(info.created_at.to_string()),
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: info.completed_at.map(|value| value.to_string()),
    }
}
