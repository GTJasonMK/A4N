use super::QueueDbError;
use crate::workflow::is_terminal_workflow_status;
use a4n_core::models::{generation::workflow_run, task::task_queue};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QuerySelect,
    Set, TransactionTrait,
};

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
    let tx = db.begin().await?;
    let mut query = task_queue::Entity::find_by_id(task_id.to_string());
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let task = query
        .one(&tx)
        .await?
        .ok_or_else(|| QueueDbError::NotFound(format!("任务不存在: {task_id}")))?;
    if is_terminal_status(&task.status) {
        tx.commit().await?;
        return Ok(());
    }

    let workflow_run_id = task.workflow_run_id;
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
    active.update(&tx).await?;
    sync_workflow_active_task(&tx, workflow_run_id, task_id, status, now).await?;
    tx.commit().await?;
    Ok(())
}

async fn sync_workflow_active_task<C>(
    db: &C,
    workflow_run_id: Option<i64>,
    task_id: &str,
    status: &str,
    now: NaiveDateTime,
) -> Result<(), QueueDbError>
where
    C: ConnectionTrait,
{
    let Some(workflow_run_id) = workflow_run_id else {
        return Ok(());
    };
    let run = workflow_run::Entity::find_by_id(workflow_run_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| {
            QueueDbError::NotFound(format!("队列任务关联工作流不存在: {workflow_run_id}"))
        })?;
    if is_terminal_workflow_status(&run.status) {
        return Ok(());
    }
    let next_active_task_id = match status {
        "pending" | "running" => Some(task_id.to_string()),
        _ if run.active_task_id.as_deref() == Some(task_id) => None,
        _ => return Ok(()),
    };
    let mut active: workflow_run::ActiveModel = run.into();
    active.active_task_id = Set(next_active_task_id);
    active.updated_at = Set(now);
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
