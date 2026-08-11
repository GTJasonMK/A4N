use a4n_core::error::AppError;
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    redact_sensitive_message, validate_provider_api_key_secret_policy,
};

use crate::contracts::EmbedTextsPayload;

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    request.result_response(result, |error| service_error(&error))
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    request.validate_known_service(EMBEDDING_SERVICE_NAME)
}

pub(crate) fn validate_embedding_request(
    request: &ServiceRequest<EmbedTextsPayload>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_provider_api_key_secret_policy(
        request.payload.api_key.as_deref(),
        &request.secret_policy,
        EMBEDDING_SERVICE_NAME,
    )
}

fn service_error_code(error: &AppError) -> ServiceErrorCode {
    match error {
        AppError::ConfigError(_) | AppError::InvalidArgument(_) | AppError::NotFound(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::EmbeddingError(_) => ServiceErrorCode::ProviderError,
        _ => ServiceErrorCode::Internal,
    }
}

pub(crate) fn service_error(error: &AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(error),
        redact_sensitive_message(error.to_string()),
    )
}
