use super::QueueDbError;
use a4n_core::models::task::task_queue;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, Set};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskStatusTimestampUpdate {
    None,
    Started,
    Completed,
}

pub async fn update_task_status(
    db: &DatabaseConnection,
    task_id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<(), QueueDbError> {
    let task = task_queue::Entity::find_by_id(task_id.to_string())
        .one(db)
        .await?
        .ok_or_else(|| QueueDbError::NotFound(format!("任务不存在: {task_id}")))?;
    if is_terminal_status(&task.status) {
        return Ok(());
    }

    let mut active: task_queue::ActiveModel = task.into();
    active.status = Set(status.to_string());
    let now = Utc::now().naive_utc();
    match task_status_timestamp_update(status) {
        TaskStatusTimestampUpdate::Started => active.started_at = Set(Some(now)),
        TaskStatusTimestampUpdate::Completed => active.completed_at = Set(Some(now)),
        TaskStatusTimestampUpdate::None => {}
    }
    if let Some(msg) = error {
        active.error_message = Set(Some(msg.to_string()));
    }
    active.update(db).await?;
    Ok(())
}

pub(super) fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
}

pub(super) fn task_status_timestamp_update(status: &str) -> TaskStatusTimestampUpdate {
    match status {
        "running" => TaskStatusTimestampUpdate::Started,
        "completed" | "failed" | "cancelled" | "interrupted" => {
            TaskStatusTimestampUpdate::Completed
        }
        _ => TaskStatusTimestampUpdate::None,
    }
}
