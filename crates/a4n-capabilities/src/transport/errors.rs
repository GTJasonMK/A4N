use a4n_service_protocol::{ServiceError, ServiceErrorCode};

pub(crate) fn transport_error(error: reqwest::Error) -> ServiceError {
    ServiceError {
        code: ServiceErrorCode::ServiceUnavailable,
        message: error.to_string(),
        retryable: Some(true),
    }
}
