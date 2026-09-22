use a4n_service_protocol::{ServiceError, ServiceErrorCode, redact_sensitive_message};
use reqwest::StatusCode;

pub(crate) fn transport_error(error: reqwest::Error) -> ServiceError {
    let message = redact_sensitive_message(error.to_string());
    if let Some(status) = error.status() {
        if status == StatusCode::PAYLOAD_TOO_LARGE {
            return ServiceError::non_retryable(
                ServiceErrorCode::ConfigurationError,
                "sidecar HTTP 413 Payload Too Large：包含图片/媒体的请求体超过接收限制",
            );
        }
        if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
            return ServiceError::non_retryable(ServiceErrorCode::Unauthorized, message);
        }
        if status.is_client_error()
            && status != StatusCode::REQUEST_TIMEOUT
            && status != StatusCode::TOO_MANY_REQUESTS
        {
            return ServiceError::non_retryable(
                ServiceErrorCode::ConfigurationError,
                format!("sidecar HTTP 请求被拒绝：{message}"),
            );
        }
    }
    ServiceError::new(ServiceErrorCode::ServiceUnavailable, message, true)
}

pub(crate) fn timeout_error(operation: &str) -> ServiceError {
    ServiceError::new(
        ServiceErrorCode::ServiceUnavailable,
        format!("sidecar {operation} timeout"),
        true,
    )
}
