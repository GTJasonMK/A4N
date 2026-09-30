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

pub(super) fn workflow_run_resume_record(run: workflow_run::Model) -> WorkflowRunResumeRecord {
    WorkflowRunResumeRecord {
        id: run.id,
        workflow_kind: run.workflow_kind,
        project_id: run.project_id,
        root_project_id: run.root_project_id,
        chapter_id: run.chapter_id,
        scope_table: run.scope_table,
        scope_id: run.scope_id,
        input_fingerprint_json: run.input_fingerprint_json,
        status: run.status,
        can_resume: run.can_resume,
        resume_block_reason: run.resume_block_reason,
    }
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
    let progress = if let Some(fields) = value.as_object() {
        if fields
            .get("snapshot")
            .is_some_and(|snapshot| !snapshot.is_object())
        {
            return Err(WorkflowReadError::InvalidData(
                "progress_json.snapshot 必须是 JSON object".to_string(),
            ));
        }
        // Recovery snapshots can dwarf public progress; never clone data we discard.
        let fields: serde_json::Map<String, Value> = fields
            .iter()
            .filter(|(key, _)| !matches!(key.as_str(), "snapshot" | "cancellation_active_task_id"))
            .map(|(key, value)| (key.clone(), value.clone()))
            .collect();
        // 恢复快照可以先于首条进度事件保存，此时没有公开进度。
        if fields.is_empty() {
            return Ok(None);
        }
        Value::Object(fields)
    } else {
        value.clone()
    };
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_test_support::workflow::WorkflowRunModelBuilder;
    use chrono::NaiveDate;

    #[test]
    fn workflow_checkpoint_summary_preserves_persisted_fields() {
        let created_at = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let updated_at = created_at + chrono::Duration::minutes(1);
        let completed_at = updated_at + chrono::Duration::minutes(1);
        let checkpoint = workflow_checkpoint::Model {
            id: 11,
            workflow_run_id: 12,
            step_key: "chapter.analysis".to_string(),
            step_order: 4,
            status: WORKFLOW_STATUS_COMPLETED.to_string(),
            input_fingerprint_sha256: Some("fingerprint".to_string()),
            input_fingerprint_json: Some(json!({"chapter_id": 13})),
            checkpoint_json: Some(json!({"result": "saved"})),
            artifacts_json: Some(json!({"profile_ids": [14]})),
            generation_run_ids_json: Some(json!([15])),
            error_message: None,
            created_at,
            updated_at,
            completed_at: Some(completed_at),
        };
        let summary = workflow_checkpoint_summary(checkpoint.clone());

        assert_eq!(summary.id, checkpoint.id);
        assert_eq!(summary.workflow_run_id, checkpoint.workflow_run_id);
        assert_eq!(summary.step_key, checkpoint.step_key);
        assert_eq!(summary.step_order, checkpoint.step_order);
        assert_eq!(summary.status, checkpoint.status);
        assert_eq!(
            summary.input_fingerprint_sha256,
            checkpoint.input_fingerprint_sha256
        );
        assert_eq!(
            summary.input_fingerprint_json,
            checkpoint.input_fingerprint_json
        );
        assert_eq!(summary.checkpoint_json, checkpoint.checkpoint_json);
        assert_eq!(summary.artifacts_json, checkpoint.artifacts_json);
        assert_eq!(
            summary.generation_run_ids_json,
            checkpoint.generation_run_ids_json
        );
        assert_eq!(summary.error_message, checkpoint.error_message);
        assert_eq!(summary.created_at, created_at.and_utc().to_rfc3339());
        assert_eq!(summary.updated_at, updated_at.and_utc().to_rfc3339());
        assert_eq!(
            summary.completed_at,
            Some(completed_at.and_utc().to_rfc3339())
        );
    }

    #[test]
    fn workflow_resume_record_does_not_parse_recovery_snapshot() {
        let created_at = NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        let run = WorkflowRunModelBuilder::new(
            17,
            "chapter:12",
            "chapter_analysis",
            WORKFLOW_STATUS_INTERRUPTED,
            created_at,
        )
        .with_project_scope(Some(10), Some(9))
        .with_scope("chapter", 12)
        .with_chapter_id(12)
        .with_input_fingerprint(json!({"source": 1}))
        .with_progress(json!({"snapshot": null}))
        .with_resumability(true, None)
        .build();
        let resume = workflow_run_resume_record(run);

        assert_eq!(resume.id, 17);
        assert_eq!(resume.workflow_kind, "chapter_analysis");
        assert_eq!(resume.project_id, Some(10));
        assert_eq!(resume.root_project_id, Some(9));
        assert_eq!(resume.chapter_id, Some(12));
        assert_eq!(resume.scope_table.as_deref(), Some("chapter"));
        assert_eq!(resume.scope_id, Some(12));
        assert_eq!(resume.input_fingerprint_json, Some(json!({"source": 1})));
        assert_eq!(resume.status, WORKFLOW_STATUS_INTERRUPTED);
        assert!(resume.can_resume);
        assert_eq!(resume.resume_block_reason, None);
    }
}
