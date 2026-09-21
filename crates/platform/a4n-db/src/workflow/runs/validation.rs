use super::*;

pub(in crate::workflow) fn validate_workflow_run_input(
    input: &UpsertWorkflowRunInput,
) -> Result<(), WorkflowValidationError> {
    required_text(input.business_key.clone(), "business_key")?;
    required_text(input.workflow_kind.clone(), "workflow_kind")?;
    validate_workflow_status(&input.status)?;
    if input.project_id.is_some_and(|id| id <= 0) {
        return Err(WorkflowValidationError("project_id 必须大于 0".to_string()));
    }
    if input.root_project_id.is_some_and(|id| id <= 0) {
        return Err(WorkflowValidationError(
            "root_project_id 必须大于 0".to_string(),
        ));
    }
    if input.chapter_id.is_some_and(|id| id <= 0) {
        return Err(WorkflowValidationError("chapter_id 必须大于 0".to_string()));
    }
    validate_workflow_status_fields(
        &input.status,
        input.can_resume,
        input.resume_block_reason.as_deref(),
        input.error_message.as_deref(),
    )?;
    Ok(())
}

pub(in crate::workflow) fn validate_workflow_status_fields(
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
    error_message: Option<&str>,
) -> Result<(), WorkflowValidationError> {
    if can_resume
        && !matches!(
            status,
            WORKFLOW_STATUS_RUNNING
                | WORKFLOW_STATUS_FAILED
                | WORKFLOW_STATUS_CANCELLED
                | WORKFLOW_STATUS_INTERRUPTED
        )
    {
        return Err(WorkflowValidationError(format!(
            "{status} 工作流不能标记为可恢复"
        )));
    }
    if matches!(
        status,
        WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_INTERRUPTED
    ) && error_message.and_then(non_empty_text).is_none()
    {
        return Err(WorkflowValidationError(format!(
            "{status} 工作流必须提供 error_message"
        )));
    }
    if !can_resume
        && !matches!(status, WORKFLOW_STATUS_RUNNING | WORKFLOW_STATUS_COMPLETED)
        && resume_block_reason.and_then(non_empty_text).is_none()
    {
        return Err(WorkflowValidationError(format!(
            "不可恢复的 {status} 工作流必须提供 resume_block_reason"
        )));
    }
    Ok(())
}

pub(in crate::workflow) fn workflow_fingerprint_matches(
    left: Option<&Value>,
    right: Option<&Value>,
) -> bool {
    left == right
}

pub(in crate::workflow) fn should_update_existing_run(
    existing: &workflow_run::Model,
    incoming_status: &str,
) -> bool {
    if existing.status != WORKFLOW_STATUS_RUNNING {
        return incoming_status == existing.status;
    }
    true
}

pub(in crate::workflow) fn should_resume_existing_run(
    existing: &workflow_run::Model,
    incoming_status: &str,
) -> bool {
    incoming_status == WORKFLOW_STATUS_RUNNING
        && existing.can_resume
        && matches!(
            existing.status.as_str(),
            WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED | WORKFLOW_STATUS_INTERRUPTED
        )
}

pub(in crate::workflow) fn requires_explicit_interrupted_resume(
    existing: &workflow_run::Model,
    incoming_status: &str,
) -> bool {
    existing.status == WORKFLOW_STATUS_INTERRUPTED
        && existing.can_resume
        && incoming_status != WORKFLOW_STATUS_INTERRUPTED
}

pub(in crate::workflow) fn completed_at_for_status_value(
    status: &str,
    now: chrono::NaiveDateTime,
) -> Option<chrono::NaiveDateTime> {
    (is_terminal_workflow_status(status) || status == WORKFLOW_STATUS_INTERRUPTED).then_some(now)
}

pub(in crate::workflow) fn ensure_workflow_status_transition(
    current: &str,
    requested: &str,
    allow_terminal_resume: bool,
) -> Result<(), WorkflowWriteError> {
    if current == WORKFLOW_STATUS_RUNNING
        || current == requested
        || (current == WORKFLOW_STATUS_INTERRUPTED && requested == WORKFLOW_STATUS_CANCELLED)
        || (allow_terminal_resume
            && requested == WORKFLOW_STATUS_RUNNING
            && matches!(
                current,
                WORKFLOW_STATUS_INTERRUPTED | WORKFLOW_STATUS_FAILED | WORKFLOW_STATUS_CANCELLED
            ))
    {
        return Ok(());
    }
    Err(WorkflowWriteError::StatusConflict {
        current: current.to_string(),
        operation: format!("将工作流状态更新为 {requested}"),
    })
}

pub(in crate::workflow) fn ensure_workflow_write_open(
    current: &str,
    operation: &str,
) -> Result<(), WorkflowWriteError> {
    if current != WORKFLOW_STATUS_RUNNING {
        return Err(WorkflowWriteError::StatusConflict {
            current: current.to_string(),
            operation: operation.to_string(),
        });
    }
    Ok(())
}
