use crate::response::{ServiceError, ServiceErrorCode};
use crate::sanitization::redact_sensitive_message;

pub(super) fn version_mismatch_error(message: String) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::VersionMismatch,
        message: redact_sensitive_message(message),
        retryable: Some(false),
    }
}

pub(super) fn unauthorized_error(message: String) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::Unauthorized,
        message: redact_sensitive_message(message),
        retryable: Some(false),
    }
}

pub(super) fn invalid_request_context_error(message: impl Into<String>) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::ConfigurationError,
        message: redact_sensitive_message(format!(
            "invalid service request context: {}",
            message.into()
        )),
        retryable: Some(false),
    }
}
