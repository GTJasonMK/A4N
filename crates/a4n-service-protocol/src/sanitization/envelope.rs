use crate::artifacts::{ArtifactValidationError, validate_artifact_refs};
use crate::events::CapabilityTaskEvent;
use crate::response::ServiceResponse;

use super::message::redact_sensitive_message;

pub fn sanitize_service_response<T>(
    response: &mut ServiceResponse<T>,
) -> Result<(), ArtifactValidationError> {
    validate_artifact_refs(&response.artifact_refs)?;
    if let Some(error) = response.error.as_mut() {
        error.message = redact_sensitive_message(&error.message);
    }
    Ok(())
}

pub fn sanitize_capability_task_event(event: &mut CapabilityTaskEvent) {
    if let Some(error) = event.error.as_mut() {
        error.message = redact_sensitive_message(&error.message);
    }
}
