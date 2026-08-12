use a4n_api_contract::task::{TaskInfoSummary, TaskProgressSummary};
use a4n_queue::priority::TaskInfo;

pub(in crate::actions::task_records) fn task_info_summary(info: &TaskInfo) -> TaskInfoSummary {
    TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| TaskProgressSummary {
            current: progress.current,
            total: progress.total,
            message: progress.message.clone(),
            meta: progress.meta.clone(),
        }),
        created_at: info.created_at.to_string(),
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: info.completed_at.map(|value| value.to_string()),
    }
}
