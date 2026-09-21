use super::errors::WorkflowValidationError;
use super::status::WORKFLOW_STATUSES;

pub(in crate::workflow) fn required_text(
    value: String,
    label: &str,
) -> Result<String, WorkflowValidationError> {
    non_empty_text(&value).ok_or_else(|| WorkflowValidationError(format!("{label} 不能为空")))
}

pub(in crate::workflow) fn non_empty_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

pub(in crate::workflow) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value.and_then(|value| non_empty_text(&value))
}

pub(in crate::workflow) fn validate_workflow_status(
    status: &str,
) -> Result<(), WorkflowValidationError> {
    if WORKFLOW_STATUSES.contains(&status) {
        Ok(())
    } else {
        Err(WorkflowValidationError(format!(
            "不支持的工作流状态: {status}"
        )))
    }
}

pub(in crate::workflow) fn validate_positive_i64(
    value: i64,
    field: &str,
) -> Result<(), WorkflowValidationError> {
    if value <= 0 {
        Err(WorkflowValidationError(format!("{field} 必须大于 0")))
    } else {
        Ok(())
    }
}
