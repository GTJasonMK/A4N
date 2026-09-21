//! workflow 与 task 转换 helper。

use super::*;

pub(in crate::context) fn task_info_summary(
    info: &a4n_queue::priority::TaskInfo,
) -> a4n_api_contract::task::TaskInfoSummary {
    a4n_api_contract::task::TaskInfoSummary {
        id: info.id.clone(),
        task_type: info.task_type.as_str().to_string(),
        priority: info.priority.as_str().to_string(),
        status: info.status.as_str().to_string(),
        root_project_id: info.root_project_id,
        source_snapshot_id: info.source_snapshot_id,
        task_scope: info.task_scope.clone(),
        progress: info.progress.as_ref().map(|progress| {
            a4n_api_contract::task::TaskProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
            }
        }),
        created_at: info.created_at.to_string(),
        started_at: info.started_at.map(|value| value.to_string()),
        completed_at: info.completed_at.map(|value| value.to_string()),
    }
}

pub(in crate::context) fn workflow_run_detail_to_api(
    detail: crate::workflow::WorkflowRunDetailSummary,
) -> a4n_api_contract::workflow::WorkflowRunDetailSummary {
    a4n_workflow_core::workflow_run_detail_to_api(detail)
}

pub(in crate::context) fn active_task_lookup_record(
    persisted: PersistedTaskInfo,
) -> Result<ActiveTaskLookupRecord, AppError> {
    let task = persisted.task;
    let progress =
        task.progress
            .as_ref()
            .map(|progress| a4n_api_contract::task::TaskProgressSummary {
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
            });
    let progress_events = task
        .progress
        .as_ref()
        .map(
            |progress| a4n_api_contract::workflow::WorkflowProgressEvent {
                id: format!("{}-snapshot", task.id),
                current: progress.current,
                total: progress.total,
                message: progress.message.clone(),
                meta: progress.meta.clone(),
                timestamp: progress
                    .updated_at
                    .clone()
                    .unwrap_or_else(|| task.started_at.unwrap_or(task.created_at).to_string()),
            },
        )
        .into_iter()
        .collect();
    Ok(ActiveTaskLookupRecord {
        task_id: task.id.clone(),
        project_id: task.project_id,
        root_project_id: task.root_project_id,
        status: task.status.as_str().to_string(),
        workflow: a4n_api_contract::workflow::WorkflowStateSummary {
            task_id: task.id,
            root_project_id: task.root_project_id,
            source_snapshot_id: task.source_snapshot_id,
            task_scope: task.task_scope,
            progress,
            progress_events,
        },
    })
}
