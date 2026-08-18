use a4n_core::error::AppError;
use a4n_service_protocol::{ServiceError, ServiceErrorCode, redact_sensitive_message};

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

pub(crate) fn service_error(err: AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(&err),
        redact_sensitive_message(err.to_string()),
    )
}
