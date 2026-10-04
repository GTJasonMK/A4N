use super::projections::{
    WorkflowCheckpointVideoRefs, generated_video_ids_for_cancellation_checkpoints,
};
use crate::workflow::{
    CancelWorkflowRunPersistenceResult, WORKFLOW_STATUS_CANCELLED, WORKFLOW_STATUS_COMPLETED,
    WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_SUPERSEDED, WorkflowWriteError,
    is_terminal_workflow_status, locked_workflow_run_by_id, required_text,
    trim_workflow_progress_events, validate_positive_i64,
};
use a4n_db_models::models::generation::{
    workflow_checkpoint, workflow_progress_event, workflow_run,
};
use chrono::Utc;
use sea_orm::sea_query::{Expr, extension::postgres::PgExpr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, NotSet, QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::json;

pub async fn cancel_workflow_run_persistence(
    db: &impl crate::DbExecutor,
    workflow_run_id: i64,
    reason: &str,
) -> Result<CancelWorkflowRunPersistenceResult, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let reason = required_text(reason.to_string(), "取消原因")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    if is_terminal_workflow_status(&run.status) && !run.can_resume {
        let (active_task_id, interrupted_generated_video_ids) =
            if run.status == WORKFLOW_STATUS_CANCELLED {
                let checkpoints = workflow_checkpoint::Entity::find()
                    .select_only()
                    .column(workflow_checkpoint::Column::Status)
                    .column_as(
                        Expr::col(workflow_checkpoint::Column::ArtifactsJson)
                            .get_json_field("generated_video_id"),
                        "artifact_generated_video_id",
                    )
                    .column_as(
                        Expr::col(workflow_checkpoint::Column::InputFingerprintJson)
                            .get_json_field("generated_video_id"),
                        "input_generated_video_id",
                    )
                    .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
                    .filter(workflow_checkpoint::Column::Status.eq(WORKFLOW_STATUS_CANCELLED))
                    .into_model::<WorkflowCheckpointVideoRefs>()
                    .all(&tx)
                    .await
                    .map_err(WorkflowWriteError::Database)?;
                let historical_task_id = workflow_progress_event::Entity::find()
                    .select_only()
                    .column(workflow_progress_event::Column::TaskId)
                    .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
                    .filter(workflow_progress_event::Column::TaskId.is_not_null())
                    .filter(workflow_progress_event::Column::TaskId.ne(""))
                    .order_by_desc(workflow_progress_event::Column::EventIndex)
                    .into_tuple::<Option<String>>()
                    .one(&tx)
                    .await
                    .map_err(WorkflowWriteError::Database)?
                    .flatten();
                let fallback_task_id = cancellation_task_id_from_run(&tx, &run).await?;
                let active_task_id = run
                    .progress_json
                    .as_ref()
                    .and_then(|progress| progress.get("cancellation_active_task_id"))
                    .and_then(serde_json::Value::as_str)
                    .and_then(|task_id| (!task_id.trim().is_empty()).then(|| task_id.to_string()))
                    .or(historical_task_id)
                    .or(fallback_task_id)
                    .or_else(|| run.active_task_id.clone());
                (
                    active_task_id,
                    generated_video_ids_for_cancellation_checkpoints(&checkpoints),
                )
            } else {
                (run.active_task_id.clone(), Vec::new())
            };
        let result = CancelWorkflowRunPersistenceResult {
            workflow_run_id,
            active_task_id,
            interrupted_generated_video_ids,
            already_terminal: true,
            cancelled: run.status == WORKFLOW_STATUS_CANCELLED,
        };
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(result);
    }
    let checkpoints = workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::Status)
        .column_as(
            Expr::col(workflow_checkpoint::Column::ArtifactsJson)
                .get_json_field("generated_video_id"),
            "artifact_generated_video_id",
        )
        .column_as(
            Expr::col(workflow_checkpoint::Column::InputFingerprintJson)
                .get_json_field("generated_video_id"),
            "input_generated_video_id",
        )
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(workflow_run_id))
        .filter(workflow_checkpoint::Column::Status.is_not_in([
            WORKFLOW_STATUS_COMPLETED,
            WORKFLOW_STATUS_FAILED,
            WORKFLOW_STATUS_CANCELLED,
            WORKFLOW_STATUS_SUPERSEDED,
        ]))
        .into_model::<WorkflowCheckpointVideoRefs>()
        .all(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    let active_task_id = cancellation_task_id_from_run(&tx, &run).await?;
    let result = CancelWorkflowRunPersistenceResult {
        workflow_run_id,
        active_task_id,
        interrupted_generated_video_ids: generated_video_ids_for_cancellation_checkpoints(
            &checkpoints,
        ),
        already_terminal: false,
        cancelled: true,
    };

    let now = Utc::now().naive_utc();
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
        .filter(workflow_checkpoint::Column::Status.is_not_in([
            WORKFLOW_STATUS_COMPLETED,
            WORKFLOW_STATUS_FAILED,
            WORKFLOW_STATUS_CANCELLED,
            WORKFLOW_STATUS_SUPERSEDED,
        ]))
        .exec(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;

    let mut progress_json = json!({
        "current": 1,
        "total": 1,
        "message": reason,
        "percent": 100.0,
        "meta": null,
        "updated_at": now.to_string(),
    });
    if let Some(snapshot) = run
        .progress_json
        .as_ref()
        .and_then(|value| value.get("snapshot"))
    {
        progress_json["snapshot"] = snapshot.clone();
    }
    progress_json["cancellation_active_task_id"] = json!(result.active_task_id.clone());
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

async fn cancellation_task_id_from_run(
    tx: &sea_orm::DatabaseTransaction,
    run: &workflow_run::Model,
) -> Result<Option<String>, WorkflowWriteError> {
    let active_task_id = run
        .progress_json
        .as_ref()
        .and_then(|progress| progress.get("cancellation_active_task_id"))
        .and_then(serde_json::Value::as_str)
        .or(run.active_task_id.as_deref())
        .map(str::trim)
        .filter(|task_id| !task_id.is_empty());
    if let Some(task_id) = active_task_id {
        return Ok(Some(task_id.to_string()));
    }
    let cancellation_message = run.error_message.as_deref().or_else(|| {
        run.progress_json
            .as_ref()
            .and_then(|progress| progress.get("message"))
            .and_then(serde_json::Value::as_str)
    });
    let Some(cancellation_message) = cancellation_message else {
        return Ok(None);
    };
    Ok(workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(run.id))
        .filter(workflow_progress_event::Column::Message.eq(cancellation_message))
        .order_by_desc(workflow_progress_event::Column::EventIndex)
        .one(tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .and_then(|event| event.task_id)
        .filter(|task_id| !task_id.trim().is_empty()))
}
