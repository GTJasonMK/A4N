use super::projections::{
    generated_video_ids_for_running_checkpoints, is_terminal_workflow_status,
    normalize_cancel_reason,
};
use crate::workflow::{
    CancelWorkflowRunPersistenceResult, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_RUNNING,
    WorkflowWriteError, trim_workflow_progress_events, validate_positive_i64, workflow_checkpoints,
    workflow_run_by_id,
};
use a4n_core::models::generation::{workflow_checkpoint, workflow_progress_event, workflow_run};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, NotSet, QueryFilter,
    QueryOrder, Set, TransactionTrait,
};
use serde_json::json;

pub async fn cancel_workflow_run_persistence(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    reason: &str,
) -> Result<CancelWorkflowRunPersistenceResult, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let reason = normalize_cancel_reason(reason);
    let run = workflow_run_by_id(db, workflow_run_id).await?;
    let checkpoints = workflow_checkpoints(db, workflow_run_id).await?;
    let interrupted_generated_video_ids = generated_video_ids_for_running_checkpoints(&checkpoints);
    let result = CancelWorkflowRunPersistenceResult {
        workflow_run_id,
        active_task_id: run.active_task_id.clone(),
        interrupted_generated_video_ids,
        already_terminal: is_terminal_workflow_status(&run.status),
    };
    if result.already_terminal {
        return Ok(result);
    }

    let now = Utc::now().naive_utc();
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    workflow_checkpoint::Entity::update_many()
        .col_expr(
            workflow_checkpoint::Column::Status,
            sea_orm::sea_query::Expr::value(WORKFLOW_STATUS_CANCELLED),
        )
        .col_expr(
            workflow_checkpoint::Column::ErrorMessage,
            sea_orm::sea_query::Expr::value(Some(reason.clone())),
        )
        .col_expr(
            workflow_checkpoint::Column::UpdatedAt,
            sea_orm::sea_query::Expr::value(now),
        )
        .col_expr(
            workflow_checkpoint::Column::CompletedAt,
            sea_orm::sea_query::Expr::value(Some(now)),
        )
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_RUNNING))
        .exec(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;

    let progress_json = json!({
        "current": 1,
        "total": 1,
        "message": reason,
        "percent": 100.0,
        "meta": null,
        "updated_at": now.to_string(),
    });
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(WORKFLOW_STATUS_CANCELLED.to_string());
    active.can_resume = Set(false);
    active.resume_block_reason = Set(Some(reason.clone()));
    active.error_message = Set(Some(reason.clone()));
    active.active_task_id = Set(None);
    active.progress_json = Set(Some(progress_json));
    active.updated_at = Set(now);
    active.completed_at = Set(Some(now));
    active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;

    let event_index = workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_desc(workflow_progress_event::Column::EventIndex)
        .one(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .map(|event| {
            event.event_index.checked_add(1).ok_or_else(|| {
                WorkflowWriteError::InvalidData("工作流取消事件序号超出 i32 范围".to_string())
            })
        })
        .transpose()?
        .unwrap_or(1);
    workflow_progress_event::ActiveModel {
        id: NotSet,
        workflow_run_id: Set(workflow_run_id),
        task_id: Set(result.active_task_id.clone()),
        event_index: Set(event_index),
        current: Set(1),
        total: Set(1),
        message: Set(reason),
        percent: Set(100.0),
        meta_json: Set(None),
        created_at: Set(now),
    }
    .insert(&tx)
    .await
    .map_err(WorkflowWriteError::Database)?;
    trim_workflow_progress_events(&tx, workflow_run_id).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(result)
}
