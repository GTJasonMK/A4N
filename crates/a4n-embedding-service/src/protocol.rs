use a4n_domain::ProviderError as AppError;
use a4n_service_protocol::{
    EMBEDDING_SERVICE_NAME, EmbedTextsPayload, ServiceError, ServiceErrorCode, ServiceRequest,
    redact_sensitive_message,
};

pub(crate) fn validate_embedding_request(
    request: &ServiceRequest<EmbedTextsPayload>,
) -> Result<(), ServiceError> {
    request
        .validate_known_provider_request(EMBEDDING_SERVICE_NAME, request.payload.api_key.as_deref())
}

fn service_error_code(error: &AppError) -> ServiceErrorCode {
    match error {
        AppError::Configuration(_) | AppError::InvalidArgument(_) | AppError::NotFound(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::Embedding(_) => ServiceErrorCode::ProviderError,
        _ => ServiceErrorCode::Internal,
    }
}

pub(crate) fn service_error(error: AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(&error),
        redact_sensitive_message(error.to_string()),
    )
}
