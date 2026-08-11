use a4n_core::error::AppError;
use a4n_service_protocol::{
    RAG_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    redact_sensitive_message,
};

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    request.result_response(result, |err| service_error(&err))
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    request.validate_known_service(RAG_SERVICE_NAME)
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
