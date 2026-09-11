use a4n_error::AppError;
use a4n_service_protocol::{
    PROMPT_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    redact_sensitive_message,
};

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    request.result_response(result, |err| {
        ServiceError::non_retryable(
            service_error_code(&err),
            redact_sensitive_message(err.to_string()),
        )
    })
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    request.validate_known_service(PROMPT_SERVICE_NAME)
}

fn service_error_code(err: &AppError) -> ServiceErrorCode {
    match err {
        AppError::InvalidArgument(_) => ServiceErrorCode::ConfigurationError,
        AppError::ConfigError(_) => ServiceErrorCode::ConfigurationError,
        AppError::NotFound(_) => ServiceErrorCode::ConfigurationError,
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        _ => ServiceErrorCode::Internal,
    }
}
