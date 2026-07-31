use a4n_service_protocol::{ServiceError, ServiceErrorCode, redact_sensitive_message};

pub(crate) fn transport_error(error: reqwest::Error) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::ServiceUnavailable,
        redact_sensitive_message(error.to_string()),
        true,
    )
}

pub(crate) fn timeout_error(operation: &str) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::ServiceUnavailable,
        format!("sidecar {operation} timeout"),
        true,
    )
}
