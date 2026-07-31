use crate::contracts::{VideoPollPayload, VideoProviderPayload};
use a4n_core::error::AppError;
use a4n_model_discovery::ModelDiscoveryRequest;
use a4n_service_protocol::{
    ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, VIDEO_SERVICE_NAME,
    redact_sensitive_message, validate_known_service_request,
    validate_provider_api_key_secret_policy,
};

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

pub(crate) fn validate_video_provider_request(
    request: &ServiceRequest<VideoProviderPayload>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_provider_secret_policy(&request.payload.api_key, request)?;
    validate_project_scope(request.payload.project_id, request.project_id, "generation")?;
    a4n_video::validate_video_provider_idempotency_key(
        request.payload.provider_idempotency_key_sha256.as_deref(),
    )
    .map_err(|error| {
        ServiceError::non_retryable(ServiceErrorCode::ConfigurationError, error.to_string())
    })?;
    Ok(())
}

pub(crate) fn validate_video_poll_request(
    request: &ServiceRequest<VideoPollPayload>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_provider_secret_policy(&request.payload.api_key, request)?;
    validate_project_scope(
        request.payload.project_id,
        request.project_id,
        "poll/status",
    )
}

fn validate_project_scope(
    payload_project_id: Option<i64>,
    envelope_project_id: Option<i64>,
    operation: &str,
) -> Result<(), ServiceError> {
    if payload_project_id.is_some_and(|project_id| project_id <= 0)
        || payload_project_id != envelope_project_id
    {
        return Err(ServiceError::non_retryable(
            ServiceErrorCode::ConfigurationError,
            format!("video {operation} payload project_id 必须与 request envelope 一致且为正整数"),
        ));
    }
    Ok(())
}

pub(crate) fn validate_model_discovery_request(
    request: &ServiceRequest<ModelDiscoveryRequest>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_optional_provider_secret_policy(request.payload.api_key.as_deref(), request)?;
    Ok(())
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request(request, VIDEO_SERVICE_NAME)
}

pub(crate) fn validate_provider_secret_policy<T>(
    api_key: &str,
    request: &ServiceRequest<T>,
) -> Result<(), ServiceError> {
    validate_optional_provider_secret_policy(Some(api_key), request)
}

pub(crate) fn validate_optional_provider_secret_policy<T>(
    api_key: Option<&str>,
    request: &ServiceRequest<T>,
) -> Result<(), ServiceError> {
    validate_provider_api_key_secret_policy(api_key, &request.secret_policy, VIDEO_SERVICE_NAME)
}

pub(crate) fn error_response<T, P>(
    request: &ServiceRequest<P>,
    error: ServiceError,
) -> ServiceResponse<T> {
    ServiceResponse::error(request.trace_context.trace_id.clone(), error)
}

pub(crate) fn service_error_code(err: &AppError) -> ServiceErrorCode {
    match err {
        AppError::InvalidArgument(_) | AppError::ConfigError(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        AppError::Timeout(_) => ServiceErrorCode::ServiceUnavailable,
        _ => ServiceErrorCode::ProviderError,
    }
}

pub(crate) fn service_error(err: &AppError) -> ServiceError {
    ServiceError::non_retryable(
        service_error_code(err),
        redact_sensitive_message(err.to_string()),
    )
}
