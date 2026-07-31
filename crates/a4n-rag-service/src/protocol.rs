use a4n_core::error::AppError;
use a4n_service_protocol::{
    RAG_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    redact_sensitive_message, validate_known_service_request,
};

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    ServiceResponse::from_result(request.trace_context.trace_id.clone(), result, |err| {
        service_error(&err)
    })
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request(request, RAG_SERVICE_NAME)
}

pub(crate) fn error_response<T, P>(
    request: &ServiceRequest<P>,
    error: ServiceError,
) -> ServiceResponse<T> {
    ServiceResponse::error(request.trace_context.trace_id.clone(), error)
}

fn service_error_code(err: &AppError) -> ServiceErrorCode {
    match err {
        AppError::InvalidArgument(_) | AppError::ConfigError(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::VectorStoreError(_) => ServiceErrorCode::ProviderError,
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        _ => ServiceErrorCode::Internal,
    }
}

pub(crate) fn service_error(err: &AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(err),
        redact_sensitive_message(err.to_string()),
    )
}
