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
                WORKFLOW_STATUS_INTERRUPTED
                    | WORKFLOW_STATUS_FAILED
                    | WORKFLOW_STATUS_CANCELLED
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
