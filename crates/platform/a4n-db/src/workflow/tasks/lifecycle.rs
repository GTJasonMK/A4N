use crate::workflow::{WORKFLOW_STATUS_INTERRUPTED, WORKFLOW_STATUS_RUNNING, WorkflowWriteError};
use crate::{DbExecutor, DbHandle};
use a4n_db_models::models::{generation::generation_run, generation::workflow_run};
use chrono::{NaiveDateTime, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QuerySelect};

const DURABLE_RUNNING_WORKFLOW_KIND: &str = "video_provider_job";

pub async fn recover_stale_in_process_workflows(db: &DbHandle) -> Result<u64, WorkflowWriteError> {
    let db = db.connection();
    let candidate_ids = workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .filter(workflow_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .filter(workflow_run::Column::WorkflowKind.ne(DURABLE_RUNNING_WORKFLOW_KIND))
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    if candidate_ids.is_empty() {
        return Ok(0);
    }

    let tx = DbExecutor::begin_db_transaction(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    let runs = workflow_run::Entity::find()
        .select_only()
        .column(workflow_run::Column::Id)
        .column(workflow_run::Column::CanResume)
        .filter(workflow_run::Column::Id.is_in(candidate_ids))
        .filter(workflow_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .filter(workflow_run::Column::WorkflowKind.ne(DURABLE_RUNNING_WORKFLOW_KIND))
        .lock_exclusive()
        .into_tuple::<(i64, bool)>()
        .all(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    let recovered_count = runs.iter().try_fold(0_u64, |count, _| {
        count.checked_add(1).ok_or_else(|| {
            WorkflowWriteError::InvalidData("待恢复运行中工作流数量超出 u64 范围".to_string())
        })
    })?;
    let (resumable_runs, non_resumable_runs): (Vec<_>, Vec<_>) =
        runs.into_iter().partition(|(_, can_resume)| *can_resume);
    let now = Utc::now().naive_utc();
    mark_workflows_interrupted(
        &tx,
        &resumable_runs
            .iter()
            .map(|(run_id, _)| *run_id)
            .collect::<Vec<_>>(),
        true,
        "应用重启后中断未完成的内存任务，可从最近有效检查点继续",
        now,
    )
    .await?;
    mark_workflows_interrupted(
        &tx,
        &non_resumable_runs
            .iter()
            .map(|(run_id, _)| *run_id)
            .collect::<Vec<_>>(),
        false,
        "应用重启后中断未完成的内存派生流程",
        now,
    )
    .await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(recovered_count)
}

async fn mark_workflows_interrupted(
    db: &crate::DbTransaction,
    workflow_run_ids: &[i64],
    can_resume: bool,
    error_message: &str,
    now: NaiveDateTime,
) -> Result<(), WorkflowWriteError> {
    if workflow_run_ids.is_empty() {
        return Ok(());
    }
    let generation_error_message = format!("所属 workflow 在应用重启后中断：{error_message}");
    workflow_run::Entity::update_many()
        .col_expr(
            workflow_run::Column::Status,
            Expr::value(WORKFLOW_STATUS_INTERRUPTED.to_string()),
        )
        .col_expr(
            workflow_run::Column::ActiveTaskId,
            Expr::value(Option::<i64>::None),
        )
        .col_expr(workflow_run::Column::CanResume, Expr::value(can_resume))
        .col_expr(
            workflow_run::Column::ResumeBlockReason,
            Expr::value(Option::<String>::None),
        )
        .col_expr(
            workflow_run::Column::ErrorMessage,
            Expr::value(Some(error_message.to_string())),
        )
        .col_expr(workflow_run::Column::UpdatedAt, Expr::value(now))
        .col_expr(workflow_run::Column::CompletedAt, Expr::value(Some(now)))
        .filter(workflow_run::Column::Id.is_in(workflow_run_ids.iter().copied()))
        .filter(workflow_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .exec(db)
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
        .filter(generation_run::Column::WorkflowRunId.is_in(workflow_run_ids.iter().copied()))
        .filter(generation_run::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .exec(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    Ok(())
}
