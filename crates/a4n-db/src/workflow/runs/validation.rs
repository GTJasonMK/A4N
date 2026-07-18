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
    if incoming_status != WORKFLOW_STATUS_RUNNING {
        return true;
    }
    !matches!(
        existing.status.as_str(),
        WORKFLOW_STATUS_COMPLETED | WORKFLOW_STATUS_SUPERSEDED
    )
}

pub(in crate::workflow) fn completed_at_for_status_value(
    status: &str,
    now: chrono::NaiveDateTime,
) -> Option<chrono::NaiveDateTime> {
    matches!(
        status,
        WORKFLOW_STATUS_COMPLETED
            | WORKFLOW_STATUS_FAILED
            | WORKFLOW_STATUS_CANCELLED
            | WORKFLOW_STATUS_SUPERSEDED
    )
    .then_some(now)
}
