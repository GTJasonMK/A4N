use a4n_error::AppError;
use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, redact_sensitive_message,
};

pub(super) fn error_response<T>(
    request: &ServiceRequest<impl Sized>,
    error: ServiceError,
) -> ServiceResponse<T> {
    ServiceResponse::error(request.trace_context.trace_id.clone(), error)
}

fn service_error_code(err: &AppError) -> ServiceErrorCode {
    match err {
        AppError::ConfigError(_) | AppError::InvalidArgument(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::Timeout(_) => ServiceErrorCode::ServiceUnavailable,
        AppError::LlmError(_) => ServiceErrorCode::ProviderError,
        _ => ServiceErrorCode::Internal,
    }
}

pub(super) fn service_error(err: AppError) -> ServiceError {
    ServiceError::new(
        service_error_code(&err),
        redact_sensitive_message(err.to_string()),
        matches!(err, AppError::Timeout(_) | AppError::LlmError(_)),
    )
}
