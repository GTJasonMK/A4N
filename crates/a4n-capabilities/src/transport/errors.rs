use a4n_service_protocol::{ServiceError, ServiceErrorCode, redact_sensitive_message};

pub(crate) fn transport_error(error: reqwest::Error) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::ServiceUnavailable,
        message: redact_sensitive_message(error.to_string()),
        retryable: Some(true),
    }
}

pub(crate) fn timeout_error(operation: &str) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::ServiceUnavailable,
        message: format!("sidecar {operation} timeout"),
        retryable: Some(true),
    }
}
