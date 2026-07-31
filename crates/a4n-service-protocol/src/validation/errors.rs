use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;

pub(super) fn version_mismatch_error(message: String) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::VersionMismatch,
        redact_sensitive_message(message),
    )
}

pub(super) fn unauthorized_error(message: String) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::Unauthorized,
        redact_sensitive_message(message),
    )
}

pub(super) fn invalid_request_context_error(message: impl Into<String>) -> ServiceError {
    ServiceError::non_retryable(
        ServiceErrorCode::ConfigurationError,
        redact_sensitive_message(format!(
            "invalid service request context: {}",
            message.into()
        )),
    )
}
