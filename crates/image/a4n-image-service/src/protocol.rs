use a4n_domain::ProviderError as AppError;
use a4n_image_contract::IMAGE_SERVICE_NAME;
use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, redact_sensitive_message,
};

use crate::contracts::ImageProviderPayload;

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    request.result_response(result, service_error)
}

pub(crate) fn validate_image_request(
    request: &ServiceRequest<ImageProviderPayload>,
) -> Result<(), ServiceError> {
    request.validate_known_provider_request(IMAGE_SERVICE_NAME, Some(&request.payload.api_key))
}

fn service_error_code(error: &AppError) -> ServiceErrorCode {
    match error {
        AppError::InvalidArgument(_) | AppError::Configuration(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::Timeout(_) => ServiceErrorCode::ServiceUnavailable,
        _ => ServiceErrorCode::ProviderError,
    }
}

pub(crate) fn service_error(error: AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(&error),
        redact_sensitive_message(error.to_string()),
    )
}
