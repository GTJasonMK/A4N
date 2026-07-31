use super::*;

use super::queries::{locked_latest_workflow_run_by_business_key_model, locked_workflow_run_by_id};
use super::validation::{
    completed_at_for_status_value, ensure_workflow_status_transition, ensure_workflow_write_open,
    requires_explicit_interrupted_resume, should_update_existing_run, validate_workflow_run_input,
    workflow_fingerprint_matches,
};

pub async fn upsert_workflow_run(
    db: &DatabaseConnection,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = upsert_workflow_run_in_transaction(&tx, input).await?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(run)
}

pub(super) async fn upsert_workflow_run_in_transaction(
    tx: &sea_orm::DatabaseTransaction,
    input: UpsertWorkflowRunInput,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_workflow_run_input(&input)?;
    let now = Utc::now().naive_utc();
    let existing =
        locked_latest_workflow_run_by_business_key_model(tx, &input.business_key).await?;
    if let Some(existing) = existing.as_ref()
        && workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
        && requires_explicit_interrupted_resume(existing, &input.status)
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: existing.status.clone(),
            operation: format!("将工作流状态更新为 {}", input.status),
        });
    }
    if let Some(existing) = existing.as_ref()
        && workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
        && is_terminal_workflow_status(&existing.status)
        && input.status != WORKFLOW_STATUS_RUNNING
        && existing.status != input.status
    {
        return Err(WorkflowWriteError::StatusConflict {
            current: existing.status.clone(),
            operation: format!("将工作流状态更新为 {}", input.status),
        });
    }
    if let Some(existing) = existing.as_ref()
        && workflow_fingerprint_matches(
            existing.input_fingerprint_json.as_ref(),
            input.input_fingerprint_json.as_ref(),
        )
        && should_update_existing_run(existing, &input.status)
    {
        let mut active: workflow_run::ActiveModel = existing.clone().into();
        active.workflow_kind = Set(input.workflow_kind);
        active.project_id = Set(input.project_id);
        active.root_project_id = Set(input.root_project_id);
        active.source_snapshot_id = Set(input.source_snapshot_id);
        active.scope_table = Set(normalize_optional_text(input.scope_table));
        active.scope_id = Set(input.scope_id);
        active.chapter_id = Set(input.chapter_id);
        active.mode = Set(normalize_optional_text(input.mode));
        active.status = Set(input.status.clone());
        active.current_step_key = Set(normalize_optional_text(input.current_step_key));
        active.active_task_id = Set(normalize_optional_text(input.active_task_id));
        active.can_resume = Set(input.can_resume);
        active.resume_block_reason = Set(normalize_optional_text(input.resume_block_reason));
        active.error_message = Set(normalize_optional_text(input.error_message));
        active.updated_at = Set(now);
        active.completed_at = Set(completed_at_for_status_value(&input.status, now));
        return active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database);
    }

    if let Some(existing) = existing
        && !is_terminal_workflow_status(&existing.status)
    {
        let mut active: workflow_run::ActiveModel = existing.into();
        active.status = Set(WORKFLOW_STATUS_SUPERSEDED.to_string());
        active.active_task_id = Set(None);
        active.can_resume = Set(false);
        active.resume_block_reason = Set(Some("输入指纹变化，旧工作流已被新运行取代".to_string()));
        active.updated_at = Set(now);
        active.completed_at = Set(Some(now));
        active
            .update(tx)
            .await
            .map_err(WorkflowWriteError::Database)?;
    }

    workflow_run::ActiveModel {
        id: NotSet,
        business_key: Set(input.business_key),
        workflow_kind: Set(input.workflow_kind),
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        scope_table: Set(normalize_optional_text(input.scope_table)),
        scope_id: Set(input.scope_id),
        chapter_id: Set(input.chapter_id),
        mode: Set(normalize_optional_text(input.mode)),
        input_fingerprint_json: Set(input.input_fingerprint_json),
        status: Set(input.status.clone()),
        current_step_key: Set(normalize_optional_text(input.current_step_key)),
        progress_json: Set(None),
        active_task_id: Set(normalize_optional_text(input.active_task_id)),
        can_resume: Set(input.can_resume),
        resume_block_reason: Set(normalize_optional_text(input.resume_block_reason)),
        error_message: Set(normalize_optional_text(input.error_message)),
        created_at: Set(now),
        updated_at: Set(now),
        completed_at: Set(completed_at_for_status_value(&input.status, now)),
    }
    .insert(tx)
    .await
    .map_err(WorkflowWriteError::Database)
}

pub async fn mark_workflow_status(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    status: &str,
    can_resume: bool,
    error_message: Option<&str>,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    validate_workflow_status(status)?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    ensure_workflow_status_transition(&run.status, status, false)?;
    if is_terminal_workflow_status(&run.status) && run.status == status {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(run);
    }
    let now = Utc::now().naive_utc();
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(status.to_string());
    active.can_resume = Set(can_resume);
    active.resume_block_reason = Set((!can_resume)
        .then(|| error_message.and_then(non_empty_text))
        .flatten());
    active.error_message = Set(error_message.and_then(non_empty_text));
    active.active_task_id = Set(None);
    active.updated_at = Set(now);
    active.completed_at = Set(completed_at_for_status_value(status, now));
    let updated = active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(updated)
}

pub async fn resume_workflow_run(
    db: &DatabaseConnection,
    workflow_run_id: i64,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    ensure_workflow_status_transition(&run.status, WORKFLOW_STATUS_RUNNING, true)?;
    if run.status == WORKFLOW_STATUS_RUNNING {
        tx.commit().await.map_err(WorkflowWriteError::Database)?;
        return Ok(run);
    }
    let mut active: workflow_run::ActiveModel = run.into();
    active.status = Set(WORKFLOW_STATUS_RUNNING.to_string());
    active.can_resume = Set(true);
    active.resume_block_reason = Set(None);
    active.error_message = Set(None);
    active.active_task_id = Set(None);
    active.updated_at = Set(Utc::now().naive_utc());
    active.completed_at = Set(None);
    let updated = active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(updated)
}

pub async fn mark_workflow_current_step(
    db: &DatabaseConnection,
    workflow_run_id: i64,
    current_step_key: &str,
) -> Result<workflow_run::Model, WorkflowWriteError> {
    validate_positive_i64(workflow_run_id, "workflow_run_id")?;
    let current_step_key = required_text(current_step_key.to_string(), "current_step_key")?;
    let tx = db.begin().await.map_err(WorkflowWriteError::Database)?;
    let run = locked_workflow_run_by_id(&tx, workflow_run_id).await?;
    ensure_workflow_write_open(&run.status, "更新工作流当前步骤")?;
    let now = Utc::now().naive_utc();
    let mut active: workflow_run::ActiveModel = run.into();
    active.current_step_key = Set(Some(current_step_key));
    active.updated_at = Set(now);
    let updated = active
        .update(&tx)
        .await
        .map_err(WorkflowWriteError::Database)?;
    tx.commit().await.map_err(WorkflowWriteError::Database)?;
    Ok(updated)
}
