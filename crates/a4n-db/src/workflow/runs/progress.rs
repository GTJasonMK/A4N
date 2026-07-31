use super::*;

use super::projections::workflow_progress_event_summary;
use super::queries::locked_workflow_run_by_id;
use super::validation::ensure_workflow_write_open;

pub async fn workflow_progress_events(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    limit: usize,
) -> Result<Vec<TaskProgressEventSummary>, WorkflowReadError> {
    let events = workflow_progress_event_models(db, workflow_run_id, limit).await?;
    events
        .into_iter()
        .map(workflow_progress_event_summary)
        .collect()
}

pub async fn record_workflow_progress_event(
    db: &DatabaseConnection,
    input: WorkflowProgressEventInput<'_>,
) -> Result<workflow_progress_event::Model, WorkflowWriteError> {
    validate_positive_i64(input.workflow_run_id, "workflow_run_id")?;
    validate_workflow_progress_range(input.current, input.total)?;
    let current = input.current;
    let total = input.total;
    let stored_current = i32::try_from(current).map_err(|_| {
        WorkflowWriteError::InvalidData(format!("工作流进度 current 超出 i32 范围: {current}"))
    })?;
    let stored_total = i32::try_from(total).map_err(|_| {
        WorkflowWriteError::InvalidData(format!("工作流进度 total 超出 i32 范围: {total}"))
    })?;
    let percent = (current as f32 / total as f32) * 100.0;
    let meta_json = input
        .meta
        .map(serde_json::to_value)
        .transpose()
        .map_err(|error| WorkflowWriteError::InvalidData(error.to_string()))?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, input.workflow_run_id).await?;
    ensure_workflow_write_open(&run.status, "记录工作流进度")?;
    let now = Utc::now().naive_utc();
    let event_index = workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(input.workflow_run_id))
        .order_by_desc(workflow_progress_event::Column::EventIndex)
        .one(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?
        .map(|event| {
            event.event_index.checked_add(1).ok_or_else(|| {
                WorkflowWriteError::InvalidData("工作流进度事件序号超出 i32 范围".to_string())
            })
        })
        .transpose()?
        .unwrap_or(1);

    let progress_json = json!({
        "current": current,
        "total": total,
        "message": input.message,
        "percent": percent,
        "meta": meta_json,
        "updated_at": now.to_string(),
    });
    let mut active: workflow_run::ActiveModel = run.into();
    active.progress_json = Set(Some(progress_json));
    active.updated_at = Set(now);
    active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;

    let event = workflow_progress_event::ActiveModel {
        id: NotSet,
        workflow_run_id: Set(input.workflow_run_id),
        task_id: Set(input.task_id.and_then(non_empty_text)),
        event_index: Set(event_index),
        current: Set(stored_current),
        total: Set(stored_total),
        message: Set(input.message.to_string()),
        percent: Set(percent),
        meta_json: Set(meta_json),
        created_at: Set(now),
    }
    .insert(&tx)
    .await
    .map_err(WorkflowWriteError::Database)?;
    trim_workflow_progress_events(&tx, input.workflow_run_id).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(event)
}

pub(in crate::workflow) fn validate_workflow_progress_range(
    current: u32,
    total: u32,
) -> Result<(), WorkflowWriteError> {
    if total == 0 {
        return Err(WorkflowWriteError::InvalidData(
            "工作流进度 total 必须大于 0".to_string(),
        ));
    }
    if current > total {
        return Err(WorkflowWriteError::InvalidData(format!(
            "工作流进度 current {} 不能大于 total {}",
            current, total
        )));
    }
    Ok(())
}

pub async fn workflow_progress_event_models(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    limit: usize,
) -> Result<Vec<workflow_progress_event::Model>, WorkflowReadError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_desc(workflow_progress_event::Column::EventIndex)
        .limit(limit.clamp(1, MAX_PERSISTED_PROGRESS_EVENTS) as u64)
        .all(db)
        .await
        .map(|events| events.into_iter().rev().collect())
        .map_err(WorkflowReadError::Database)
}

pub(in crate::workflow) async fn trim_workflow_progress_events<C>(
    db: &C,
    workflow_run_id: i64,
) -> Result<(), WorkflowWriteError>
where
    C: sea_orm::ConnectionTrait,
{
    let count = workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
        .count(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    if count <= MAX_PERSISTED_PROGRESS_EVENTS as u64 {
        return Ok(());
    }
    let stale_count = count - MAX_PERSISTED_PROGRESS_EVENTS as u64;
    let stale_ids = workflow_progress_event::Entity::find()
        .filter(workflow_progress_event::Column::WorkflowRunId.eq(workflow_run_id))
        .order_by_asc(workflow_progress_event::Column::EventIndex)
        .limit(stale_count)
        .all(db)
        .await
        .map_err(WorkflowWriteError::Database)?
        .into_iter()
        .map(|event| event.id)
        .collect::<Vec<_>>();
    if stale_ids.is_empty() {
        return Ok(());
    }
    workflow_progress_event::Entity::delete_many()
        .filter(workflow_progress_event::Column::Id.is_in(stale_ids))
        .exec(db)
        .await
        .map_err(WorkflowWriteError::Database)?;
    Ok(())
}
