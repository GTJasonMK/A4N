use super::QueueDbError;
use crate::DbHandle;
use crate::workflow::is_terminal_workflow_status;
use a4n_db_models::models::{generation::workflow_run, task::task_queue};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult, QueryFilter, QueryOrder,
    QuerySelect, TransactionTrait,
};
use std::collections::BTreeSet;

#[derive(Debug, FromQueryResult)]
struct TaskStatusRecord {
    id: String,
    status: String,
    task_type: String,
    workflow_run_id: Option<i64>,
}

#[derive(Debug, FromQueryResult)]
struct WorkflowStatusRecord {
    id: i64,
    status: String,
    active_task_id: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TaskStatusTimestampUpdate {
    None,
    Started,
    Completed,
}

/// Returns false for an already-terminal row and true when a non-terminal row was updated.
pub async fn update_task_status(
    db: &DbHandle,
    task_id: &str,
    status: &str,
    error: Option<&str>,
) -> Result<bool, QueueDbError> {
    let db = db.connection();
    let tx = db.begin().await?;
    let mut query = task_queue::Entity::find_by_id(task_id.to_string())
        .select_only()
        .column(task_queue::Column::Id)
        .column(task_queue::Column::Status)
        .column(task_queue::Column::TaskType)
        .column(task_queue::Column::WorkflowRunId);
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let task = query
        .into_model::<TaskStatusRecord>()
        .one(&tx)
        .await?
        .ok_or_else(|| QueueDbError::NotFound(format!("任务不存在: {task_id}")))?;
    if is_terminal_status(&task.status) {
        if status == "running" && task.status == "cancelled" {
            return Err(QueueDbError::Cancelled);
        }
        tx.commit().await?;
        return Ok(false);
    }

    let workflow_run_id = task.workflow_run_id;
    let now = Utc::now().naive_utc();
    let mut update = task_queue::Entity::update_many()
        .col_expr(
            task_queue::Column::Status,
            sea_orm::sea_query::Expr::value(status.to_string()),
        )
        .filter(task_queue::Column::Id.eq(&task.id));
    match task_status_timestamp_update(status) {
        TaskStatusTimestampUpdate::Started => {
            update = update.col_expr(
                task_queue::Column::StartedAt,
                sea_orm::sea_query::Expr::value(Some(now)),
            );
        }
        TaskStatusTimestampUpdate::Completed => {
            update = update.col_expr(
                task_queue::Column::CompletedAt,
                sea_orm::sea_query::Expr::value(Some(now)),
            );
        }
        TaskStatusTimestampUpdate::None => {}
    }
    if let Some(msg) = error {
        update = update.col_expr(
            task_queue::Column::ErrorMessage,
            sea_orm::sea_query::Expr::value(Some(msg.to_string())),
        );
    }
    if update.exec(&tx).await?.rows_affected != 1 {
        return Err(QueueDbError::NotFound(format!("任务不存在: {task_id}")));
    }
    if task.task_type != "llm_call" {
        // Child model calls must not replace the workflow's generation-task owner.
        sync_workflow_active_task(&tx, workflow_run_id, task_id, status, now).await?;
    }
    tx.commit().await?;
    Ok(true)
}

/// 将启动恢复发现的多个普通队列任务一次性标记为 interrupted。
pub async fn interrupt_active_tasks(
    db: &DbHandle,
    task_ids: &[String],
    error: &str,
) -> Result<(), QueueDbError> {
    let db = db.connection();
    if task_ids.is_empty() {
        return Ok(());
    }
    let mut requested_ids = task_ids.to_vec();
    requested_ids.sort_unstable();
    requested_ids.dedup();

    let tx = db.begin().await?;
    let mut query = task_queue::Entity::find()
        .select_only()
        .column(task_queue::Column::Id)
        .column(task_queue::Column::Status)
        .column(task_queue::Column::TaskType)
        .column(task_queue::Column::WorkflowRunId)
        .filter(task_queue::Column::Id.is_in(requested_ids.clone()))
        .order_by_asc(task_queue::Column::Id);
    if tx.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let tasks = query.into_model::<TaskStatusRecord>().all(&tx).await?;
    if tasks.len() != requested_ids.len() {
        let found = tasks
            .iter()
            .map(|task| task.id.as_str())
            .collect::<BTreeSet<_>>();
        let missing = requested_ids
            .iter()
            .find(|task_id| !found.contains(task_id.as_str()))
            .cloned()
            .unwrap_or_else(|| requested_ids[0].clone());
        return Err(QueueDbError::NotFound(format!("任务不存在: {missing}")));
    }

    let recoverable = tasks
        .into_iter()
        .filter(|task| !is_terminal_status(&task.status))
        .collect::<Vec<_>>();
    if recoverable.is_empty() {
        tx.commit().await?;
        return Ok(());
    }
    let recoverable_ids = recoverable
        .iter()
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let now = Utc::now().naive_utc();
    let updated = task_queue::Entity::update_many()
        .col_expr(
            task_queue::Column::Status,
            sea_orm::sea_query::Expr::value("interrupted"),
        )
        .col_expr(
            task_queue::Column::CompletedAt,
            sea_orm::sea_query::Expr::value(Some(now)),
        )
        .col_expr(
            task_queue::Column::ErrorMessage,
            sea_orm::sea_query::Expr::value(Some(error.to_string())),
        )
        .filter(task_queue::Column::Id.is_in(recoverable_ids.clone()))
        .exec(&tx)
        .await?;
    if updated.rows_affected != recoverable_ids.len() as u64 {
        return Err(QueueDbError::NotFound("任务不存在".to_string()));
    }

    let workflow_ids = recoverable
        .iter()
        .filter_map(|task| task.workflow_run_id)
        .collect::<BTreeSet<_>>();
    if !workflow_ids.is_empty() {
        let runs = workflow_run::Entity::find()
            .select_only()
            .column(workflow_run::Column::Id)
            .column(workflow_run::Column::Status)
            .column(workflow_run::Column::ActiveTaskId)
            .filter(workflow_run::Column::Id.is_in(workflow_ids.iter().copied()))
            .order_by_asc(workflow_run::Column::Id)
            .lock_exclusive()
            .into_model::<WorkflowStatusRecord>()
            .all(&tx)
            .await?;
        if runs.len() != workflow_ids.len() {
            let found = runs.iter().map(|run| run.id).collect::<BTreeSet<_>>();
            let missing = workflow_ids
                .iter()
                .find(|workflow_id| !found.contains(workflow_id))
                .copied()
                .unwrap_or_default();
            return Err(QueueDbError::NotFound(format!(
                "队列任务关联工作流不存在: {missing}"
            )));
        }
        let mutable_workflow_ids = runs
            .into_iter()
            .filter(|run| !is_terminal_workflow_status(&run.status))
            .map(|run| run.id)
            .collect::<Vec<_>>();
        if !mutable_workflow_ids.is_empty() {
            let workflow_updated = workflow_run::Entity::update_many()
                .col_expr(
                    workflow_run::Column::ActiveTaskId,
                    sea_orm::sea_query::Expr::value(Option::<String>::None),
                )
                .col_expr(
                    workflow_run::Column::UpdatedAt,
                    sea_orm::sea_query::Expr::value(now),
                )
                .filter(workflow_run::Column::Id.is_in(mutable_workflow_ids))
                .filter(workflow_run::Column::ActiveTaskId.is_in(recoverable_ids))
                .exec(&tx)
                .await?;
            if workflow_updated.rows_affected > workflow_ids.len() as u64 {
                return Err(QueueDbError::NotFound(
                    "队列任务关联工作流更新数量不一致".to_string(),
                ));
            }
        }
    }
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
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::Status)
        .column(workflow_run::Column::ActiveTaskId)
        .lock_exclusive()
        .into_model::<WorkflowStatusRecord>()
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
        _ if run.active_task_id.as_deref() == Some(task_id) => {
            task_queue::Entity::find()
                .select_only()
                .column(task_queue::Column::Id)
                .filter(task_queue::Column::WorkflowRunId.eq(workflow_run_id))
                .filter(task_queue::Column::Id.ne(task_id))
                .filter(task_queue::Column::Status.is_in(["pending", "running"]))
                .order_by_asc(task_queue::Column::CreatedAt)
                .order_by_asc(task_queue::Column::Id)
                .into_tuple::<String>()
                .one(db)
                .await?
        }
        _ => return Ok(()),
    };
    let updated = workflow_run::Entity::update_many()
        .col_expr(
            workflow_run::Column::ActiveTaskId,
            sea_orm::sea_query::Expr::value(next_active_task_id),
        )
        .col_expr(
            workflow_run::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .filter(workflow_run::Column::Id.eq(run.id))
        .exec(db)
        .await?;
    if updated.rows_affected != 1 {
        return Err(QueueDbError::NotFound(format!(
            "队列任务关联工作流不存在: {workflow_run_id}"
        )));
    }
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
