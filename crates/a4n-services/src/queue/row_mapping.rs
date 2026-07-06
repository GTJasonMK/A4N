use a4n_core::error::AppError;
use a4n_core::models::task::task_queue;
use a4n_engine::queue::priority::{TaskInfo, TaskPriority, TaskProgress, TaskStatus, TaskType};

pub(super) fn task_info_from_row(row: task_queue::Model) -> Result<TaskInfo, AppError> {
    Ok(TaskInfo {
        id: row.id,
        task_type: TaskType::parse_storage(&row.task_type)?,
        priority: TaskPriority::parse_storage(&row.priority)?,
        status: TaskStatus::parse_storage(&row.status, row.error_message.as_deref())?,
        project_id: row.project_id,
        root_project_id: row.root_project_id,
        source_snapshot_id: row.source_snapshot_id,
        task_scope: row.task_scope,
        progress: row
            .progress
            .map(serde_json::from_value::<TaskProgress>)
            .transpose()?,
        created_at: row.created_at,
        started_at: row.started_at,
        completed_at: row.completed_at,
    })
}
