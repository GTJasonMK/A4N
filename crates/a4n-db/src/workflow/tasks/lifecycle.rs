use crate::workflow::{
    WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WorkflowWriteError,
    is_terminal_workflow_status, locked_workflow_run_by_id, required_text, validate_positive_i64,
};
use a4n_core::models::{generation::generation_run, generation::workflow_run, task::task_queue};
use chrono::Utc;
use sea_orm::sea_query::Expr;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DatabaseTransaction, EntityTrait,
    QueryFilter, QuerySelect, Set, TransactionTrait,
};

const DURABLE_RUNNING_WORKFLOW_KIND: &str = "video_provider_job";

pub async fn mark_recovered_task_workflow_interrupted(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    task_id: &str,
) -> Result<(), WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let task_id = required_text(task_id.to_string(), "task_id")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let Some(run) = workflow_run::Entity::find_by_id(workflow_run_id)
        .lock_exclusive()
        .one(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
    else {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(());
    };
    if is_terminal_workflow_status(&run.status) {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(());
    }

    mark_workflow_interrupted(
        &tx,
        run,
        true,
        format!("应用重启后中断任务 {task_id}，可从最近有效检查点继续"),
    )
    .await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(())
}

pub async fn recover_stale_in_process_workflows(
    db: &DatabaseConnection,
) -> Result<u64, WorkflowWriteError> {
    let runs = workflow_run::Entity::find()
        .filter(workflow_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .filter(workflow_run::Column::WorkflowKind.ne(DURABLE_RUNNING_WORKFLOW_KIND))
        .all(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    let mut recovered_count = 0_u64;
    for candidate in runs {
        let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
        let Some(run) = workflow_run::Entity::find_by_id(candidate.id)
            .lock_exclusive()
            .one(&tx)
            .await
            .map_err(WorkflowWriteError::Database)?
        else {
            tx.commit().await.map_err(WorkflowWriteError::Database)?;
            continue;
        };
        if run.status != WORKFLOW_STATUS_RUNNING
            || run.workflow_kind == DURABLE_RUNNING_WORKFLOW_KIND
        {
            tx.commit().await.map_err(WorkflowWriteError::Database)?;
            continue;
        }
        let can_resume = run.can_resume || run.workflow_kind == "chapter_analysis";
        mark_workflow_interrupted(
            &tx,
            run,
            can_resume,
            if can_resume {
                "应用重启后中断未完成的内存任务，可从最近有效检查点继续".to_string()
            } else {
                "应用重启后中断未完成的内存派生流程".to_string()
            },
        )
        .await?;
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        recovered_count = recovered_count.checked_add(1).ok_or_else(|| {
            WorkflowWriteError::InvalidData("待恢复运行中工作流数量超出 u64 范围".to_string())
        })?;
    }
    Ok(recovered_count)
}

async fn mark_workflow_interrupted(
    db: &DatabaseTransaction,
    run: workflow_run::Model,
    can_resume: bool,
    error_message: String,
) -> Result<(), WorkflowWriteError> {
    let now = Utc::now().naive_utc();
    let workflow_run_id = run.id;
    let generation_error_message = format!("所属 workflow 在应用重启后中断：{error_message}");
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(WORKFLOW_STATUS_INTERRUPTED.to_string());
    active.active_task_id = Set(None);
    active.can_resume = Set(can_resume);
    active.resume_block_reason = Set(None);
    active.error_message = Set(Some(error_message));
    active.updated_at = Set(now);
    active.completed_at = Set(Some(now));
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
            Expr::value(Some(generation_error_message)),
        )
        .col_expr(generation_run::Column::CompletedAt, Expr::value(Some(now)))
        .filter(generation_run::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(generation_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .exec(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    Ok(())
}

pub async fn sync_workflow_active_task_for_queue_status(
    db: &DatabaseConnection,
    task_id: &str,
    status: &str,
) -> Result<(), WorkflowWriteError> {
    let task_id = required_text(task_id.to_string(), "task_id")?;
    let status = required_text(status.to_string(), "status")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let Some(task) = task_queue::Entity::find_by_id(task_id.clone())
        .one(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
    else {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(());
    };
    let Some(workflow_run_id) = task.workflow_run_id else {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(());
    };
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    if is_terminal_workflow_status(&run.status) {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(());
    }
    let next_active_task_id = match status.as_str() {
        "pending" | "running" => Some(task_id),
        _ if run.active_task_id.as_deref() == Some(task_id.as_str()) => None,
        _ => {
            tx.commit().await.map_err(WorkflowWriteError::Database)?;
            return Ok(());
        }
    };
    let mut active: workflow_run::ActiveModel = run.into();
    active.active_task_id = Set(next_active_task_id);
    active.updated_at = Set(Utc::now().naive_utc());
    active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(())
}
