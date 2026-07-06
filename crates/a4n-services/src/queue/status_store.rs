use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::task::task_queue;
use chrono::Utc;
use sea_orm::*;

pub(super) async fn update_task_status_by_db(
    db: Arc<DatabaseConnection>,
    task_id: String,
    status: &str,
    error: Option<String>,
) -> Result<(), AppError> {
    let task = task_queue::Entity::find_by_id(task_id.clone())
        .one(db.as_ref())
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("任务不存在: {task_id}")))?;

    if is_terminal_status(&task.status) {
        return Ok(());
    }

    let mut active: task_queue::ActiveModel = task.into();
    active.status = Set(status.to_string());
    let now = Utc::now().naive_utc();

    match status {
        "running" => {
            active.started_at = Set(Some(now));
        }
        "completed" | "failed" | "cancelled" | "interrupted" => {
            active.completed_at = Set(Some(now));
        }
        _ => {}
    }

    if let Some(msg) = error {
        active.error_message = Set(Some(msg));
    }

    active
        .update(db.as_ref())
        .await
        .map_err(AppError::Database)?;
    Ok(())
}

fn is_terminal_status(status: &str) -> bool {
    matches!(status, "completed" | "failed" | "cancelled" | "interrupted")
}
