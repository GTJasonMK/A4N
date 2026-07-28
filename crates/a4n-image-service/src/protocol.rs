use a4n_core::error::AppError;
use a4n_model_discovery::ModelDiscoveryRequest;
use a4n_service_protocol::{
    EgressDataType, IMAGE_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest,
    ServiceResponse, ServiceStatus, redact_sensitive_message, validate_egress_scope,
    validate_known_service_request_envelope, validate_provider_api_key_secret_policy,
};

use crate::contracts::ImageProviderPayload;

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    match result {
        Ok(payload) => ServiceResponse {
            status: ServiceStatus::Ok,
            trace_id: Some(request.trace_context.trace_id.clone()),
            usage: None,
            artifact_refs: Vec::new(),
            payload: Some(payload),
            error: None,
        },
        Err(error) => ServiceResponse {
            status: ServiceStatus::Error,
            trace_id: Some(request.trace_context.trace_id.clone()),
            usage: None,
            artifact_refs: Vec::new(),
            payload: None,
            error: Some(service_error(&error)),
        },
    }
}

pub(crate) fn validate_image_request(
    request: &ServiceRequest<ImageProviderPayload>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_optional_provider_secret_policy(Some(&request.payload.api_key), request)
}

pub(crate) fn validate_model_discovery_request(
    request: &ServiceRequest<ModelDiscoveryRequest>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_optional_provider_secret_policy(request.payload.api_key.as_deref(), request)
}

fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request_envelope(request, IMAGE_SERVICE_NAME)?;
    validate_egress_scope(
        IMAGE_SERVICE_NAME,
        EgressDataType::Image,
        &request.secret_policy.scope,
    )
    .map(|_| ())
}

fn validate_optional_provider_secret_policy<T>(
    api_key: Option<&str>,
    request: &ServiceRequest<T>,
) -> Result<(), ServiceError> {
    validate_provider_api_key_secret_policy(api_key, &request.secret_policy, IMAGE_SERVICE_NAME)
}

pub(crate) fn error_response<T, P>(
    request: &ServiceRequest<P>,
    error: ServiceError,
) -> ServiceResponse<T> {
    ServiceResponse {
        status: ServiceStatus::Error,
        trace_id: Some(request.trace_context.trace_id.clone()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: None,
        error: Some(error),
    }
}

fn service_error_code(error: &AppError) -> ServiceErrorCode {
    match error {
        AppError::InvalidArgument(_) | AppError::ConfigError(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::Timeout(_) => ServiceErrorCode::ServiceUnavailable,
        _ => ServiceErrorCode::ProviderError,
    }
}

pub(crate) fn service_error(error: &AppError) -> ServiceError {
    ServiceError {
        code: service_error_code(error),
        message: redact_sensitive_message(error.to_string()),
        retryable: Some(false),
    }
}
