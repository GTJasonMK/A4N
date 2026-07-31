use a4n_core::error::AppError;
use a4n_service_protocol::{
    PROMPT_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    redact_sensitive_message, validate_known_service_request,
};

pub(crate) fn ok_response<T>(
    request: &ServiceRequest<impl Sized>,
    payload: T,
) -> ServiceResponse<T> {
    ServiceResponse::ok(request.trace_context.trace_id.clone(), payload)
}

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    ServiceResponse::from_result(request.trace_context.trace_id.clone(), result, |err| {
        ServiceError::non_retryable(
            service_error_code(&err),
            redact_sensitive_message(err.to_string()),
        )
    })
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request(request, PROMPT_SERVICE_NAME)
}

pub(crate) fn error_response<T, P>(
    request: &ServiceRequest<P>,
    error: ServiceError,
) -> ServiceResponse<T> {
    ServiceResponse::error(request.trace_context.trace_id.clone(), error)
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
