use super::*;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PersistedWorkflowProgress {
    current: u32,
    total: u32,
    message: String,
    percent: Option<f32>,
    meta: Option<ProgressMeta>,
    updated_at: Option<String>,
}

pub(in crate::workflow) fn workflow_run_summary(
    run: workflow_run::Model,
) -> Result<WorkflowRunSummary, WorkflowReadError> {
    let progress = run
        .progress_json
        .as_ref()
        .map(workflow_run_progress_summary)
        .transpose()?
        .flatten();
    Ok(WorkflowRunSummary {
        id: run.id,
        business_key: run.business_key,
        workflow_kind: run.workflow_kind,
        project_id: run.project_id,
        root_project_id: run.root_project_id,
        source_snapshot_id: run.source_snapshot_id,
        scope_table: run.scope_table,
        scope_id: run.scope_id,
        chapter_id: run.chapter_id,
        mode: run.mode,
        input_fingerprint_json: run.input_fingerprint_json,
        status: run.status,
        current_step_key: run.current_step_key,
        progress,
        progress_json: run.progress_json,
        active_task_id: run.active_task_id,
        can_resume: run.can_resume,
        resume_block_reason: run.resume_block_reason,
        error_message: run.error_message,
        created_at: run.created_at.and_utc().to_rfc3339(),
        updated_at: run.updated_at.and_utc().to_rfc3339(),
        completed_at: run.completed_at.map(|value| value.and_utc().to_rfc3339()),
    })
}

pub(in crate::workflow) fn workflow_run_summary_for_write(
    run: workflow_run::Model,
) -> Result<WorkflowRunSummary, WorkflowWriteError> {
    workflow_run_summary(run).map_err(WorkflowWriteError::from)
}

pub(super) fn workflow_checkpoint_summary(
    checkpoint: workflow_checkpoint::Model,
) -> WorkflowCheckpointSummary {
    WorkflowCheckpointSummary {
        id: checkpoint.id,
        workflow_run_id: checkpoint.workflow_run_id,
        step_key: checkpoint.step_key,
        step_order: checkpoint.step_order,
        status: checkpoint.status,
        input_fingerprint_sha256: checkpoint.input_fingerprint_sha256,
        input_fingerprint_json: checkpoint.input_fingerprint_json,
        checkpoint_json: checkpoint.checkpoint_json,
        artifacts_json: checkpoint.artifacts_json,
        generation_run_ids_json: checkpoint.generation_run_ids_json,
        error_message: checkpoint.error_message,
        created_at: checkpoint.created_at.and_utc().to_rfc3339(),
        updated_at: checkpoint.updated_at.and_utc().to_rfc3339(),
        completed_at: checkpoint
            .completed_at
            .map(|value| value.and_utc().to_rfc3339()),
    }
}

pub(in crate::workflow) fn workflow_run_progress_summary(
    value: &Value,
) -> Result<Option<WorkflowRunProgressSummary>, WorkflowReadError> {
    let mut progress = value.clone();
    if let Some(fields) = progress.as_object_mut() {
        fields.remove("cancellation_active_task_id");
    }
    if progress.as_object().is_some_and(|fields| fields.is_empty()) {
        return Ok(None);
    }
    if let Some(fields) = progress.as_object_mut()
        && let Some(snapshot) = fields.remove("snapshot")
    {
        if !snapshot.is_object() {
            return Err(WorkflowReadError::InvalidData(
                "progress_json.snapshot 必须是 JSON object".to_string(),
            ));
        }
        // 恢复快照可以先于首条进度事件保存，此时没有公开进度。
        if fields.is_empty() {
            return Ok(None);
        }
    }
    let persisted = serde_json::from_value::<PersistedWorkflowProgress>(progress)
        .map_err(|error| WorkflowReadError::InvalidData(format!("progress_json 无效: {error}")))?;
    if persisted.total == 0 {
        return Err(WorkflowReadError::InvalidData(
            "progress_json.total 必须大于 0".to_string(),
        ));
    }
    if persisted.current > persisted.total {
        return Err(WorkflowReadError::InvalidData(format!(
            "progress_json.current {} 不能大于 total {}",
            persisted.current, persisted.total
        )));
    }
    let percent = persisted
        .percent
        .unwrap_or_else(|| (persisted.current as f32 / persisted.total as f32) * 100.0);
    if !percent.is_finite() || !(0.0..=100.0).contains(&percent) {
        return Err(WorkflowReadError::InvalidData(format!(
            "progress_json.percent 必须在 0..=100 范围内，实际为 {percent}"
        )));
    }
    Ok(Some(WorkflowRunProgressSummary {
        current: persisted.current,
        total: persisted.total,
        message: persisted.message,
        percent,
        meta: persisted.meta,
        updated_at: persisted.updated_at,
    }))
}

pub(super) fn workflow_progress_event_summary(
    event: workflow_progress_event::Model,
) -> Result<TaskProgressEventSummary, WorkflowReadError> {
    if event.current < 0 || event.total <= 0 || event.current > event.total {
        return Err(WorkflowReadError::InvalidData(format!(
            "workflow_progress_event #{} 的进度范围无效: current={}, total={}",
            event.id, event.current, event.total
        )));
    }
    if !event.percent.is_finite() || !(0.0..=100.0).contains(&event.percent) {
        return Err(WorkflowReadError::InvalidData(format!(
            "workflow_progress_event #{} 的 percent 无效: {}",
            event.id, event.percent
        )));
    }
    let meta = event
        .meta_json
        .map(serde_json::from_value)
        .transpose()
        .map_err(|error| {
            WorkflowReadError::InvalidData(format!(
                "workflow_progress_event #{} 的 meta_json 无效: {error}",
                event.id
            ))
        })?;
    Ok(TaskProgressEventSummary {
        id: event.id.to_string(),
        task_id: event.task_id,
        current: u32::try_from(event.current).map_err(|_| {
            WorkflowReadError::InvalidData(format!(
                "workflow_progress_event #{} 的 current 超出 u32 范围: {}",
                event.id, event.current
            ))
        })?,
        total: u32::try_from(event.total).map_err(|_| {
            WorkflowReadError::InvalidData(format!(
                "workflow_progress_event #{} 的 total 超出 u32 范围: {}",
                event.id, event.total
            ))
        })?,
        message: event.message,
        percent: event.percent,
        meta,
        timestamp: event.created_at.and_utc().to_rfc3339(),
    })
}
