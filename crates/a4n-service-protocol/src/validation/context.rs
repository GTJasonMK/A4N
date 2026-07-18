use crate::request::ServiceRequest;
use crate::response::ServiceError;
use crate::sanitization::contains_unredacted_sensitive_value;

use super::errors::invalid_request_context_error;

pub fn validate_service_request_context<T>(
    request: &ServiceRequest<T>,
) -> Result<(), ServiceError> {
    if request.request_id.trim().is_empty() {
        return Err(invalid_request_context_error(
            "request_id must not be blank",
        ));
    }
    if request.trace_context.trace_id.trim().is_empty() {
        return Err(invalid_request_context_error(
            "trace_context.trace_id must not be blank",
        ));
    }
    if request
        .trace_context
        .parent_span_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty())
    {
        return Err(invalid_request_context_error(
            "trace_context.parent_span_id must not be blank when present",
        ));
    }

    validate_positive_optional_id("user_id", request.user_id)?;
    validate_positive_optional_id("project_id", request.project_id)?;
    validate_positive_optional_id("root_project_id", request.root_project_id)?;

    if request.root_project_id.is_some() && request.project_id.is_none() {
        return Err(invalid_request_context_error(
            "root_project_id requires project_id",
        ));
    }

    if let Some(snapshot) = request.runtime_config_snapshot.as_ref() {
        if !snapshot.is_object() {
            return Err(invalid_request_context_error(
                "runtime_config_snapshot must be a JSON object when present",
            ));
        }
        if contains_unredacted_sensitive_value(snapshot) {
            return Err(invalid_request_context_error(
                "runtime_config_snapshot contains unredacted sensitive fields",
            ));
        }
    }

    Ok(())
}

fn validate_positive_optional_id(field: &str, value: Option<i64>) -> Result<(), ServiceError> {
    if value.is_some_and(|value| value <= 0) {
        return Err(invalid_request_context_error(format!(
            "{field} must be a positive integer when present"
        )));
    }
    Ok(())
}
