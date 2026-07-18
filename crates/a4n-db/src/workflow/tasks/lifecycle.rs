use super::projections::is_terminal_workflow_status;
use crate::workflow::{
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WorkflowWriteError, required_text,
    validate_positive_i64,
};
use a4n_core::models::{generation::generation_run, generation::workflow_run, task::task_queue};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

pub async fn mark_recovered_task_workflow_interrupted<C>(
    db: &C,
    workflow_run_id: i64,
    task_id: &str,
) -> Result<(), WorkflowWriteError>
where
    C: ConnectionTrait,
{
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let task_id = required_text(task_id.to_string(), "task_id")?;
    let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(db)
        .await
        .map_err(WorkflowWriteError::Database)?
    else {
        return Ok(());
    };
    if is_terminal_workflow_status(&run.status) {
        return Ok(());
    }

    let now = Utc::now().naive_utc();
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(WORKFLOW_STATUS_INTERRUPTED.to_string());
    active.active_task_id = Set(None);
    active.can_resume = Set(true);
    active.resume_block_reason = Set(None);
    active.error_message = Set(Some(format!(
        "应用重启后中断任务 {task_id}，可从最近有效检查点继续"
    )));
    active.updated_at = Set(now);
    active
        .update(db)
        .await
        .map_err(WorkflowWriteError::Database)?;

    generation_run::Entity::update_many()
        .col_expr(
            generation_run::Column::Status,
            Expr::value(WORKFLOW_STATUS_INTERRUPTED.to_string()),
        )
        .col_expr(
            generation_run::Column::ErrorMessage,
            Expr::value(Some(format!(
                "应用重启后中断关联队列任务 {task_id}，等待 workflow 恢复"
            ))),
        )
        .col_expr(generation_run::Column::CompletedAt, Expr::value(Some(now)))
        .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(generation_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .exec(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    Ok(())
}

pub async fn sync_workflow_active_task_for_queue_status<C>(
    db: &C,
    task_id: &str,
    status: &str,
) -> Result<(), WorkflowWriteError>
where
    C: ConnectionTrait,
{
    let task_id = required_text(task_id.to_string(), "task_id")?;
    let status = required_text(status.to_string(), "status")?;
    let Some(task) = task_queue::Entity::find_by_id(task_id.clone())
        .one(db)
        .await
        .map_err(WorkflowWriteError::Database)?
    else {
        return Ok(());
    };
    let Some(workflow_run_id) = task.workflow_run_id else {
        return Ok(());
    };
    let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
        .one(db)
        .await
        .map_err(WorkflowWriteError::Database)?
    else {
        return Ok(());
    };
    let next_active_task_id = match status.as_str() {
        "pending" | "running" => Some(task_id),
        _ if run.active_task_id.as_deref() == Some(task_id.as_str()) => None,
        _ => return Ok(()),
    };
    let mut active: workflow_run::ActiveModel = run.into();
    active.active_task_id = Set(next_active_task_id);
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    Ok(())
}
