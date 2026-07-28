use crate::contracts::{VideoPollPayload, VideoProviderPayload};
use a4n_core::error::AppError;
use a4n_model_discovery::ModelDiscoveryRequest;
use a4n_service_protocol::{
    EgressDataType, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse, ServiceStatus,
    VIDEO_SERVICE_NAME, redact_sensitive_message, validate_egress_scope,
    validate_known_service_request_envelope, validate_provider_api_key_secret_policy,
};

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
        Err(err) => ServiceResponse {
            status: ServiceStatus::Error,
            trace_id: Some(request.trace_context.trace_id.clone()),
            usage: None,
            artifact_refs: Vec::new(),
            payload: None,
            error: Some(ServiceError {
                code: service_error_code(&err),
                message: redact_sensitive_message(err.to_string()),
                retryable: Some(false),
            }),
        },
    }
}

pub(crate) fn validate_video_provider_request(
    request: &ServiceRequest<VideoProviderPayload>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_provider_secret_policy(&request.payload.api_key, request)?;
    a4n_video::validate_video_provider_idempotency_key(
        request.payload.provider_idempotency_key_sha256.as_deref(),
    )
    .map_err(|error| ServiceError {
        code: ServiceErrorCode::ConfigurationError,
        message: error.to_string(),
        retryable: Some(false),
    })?;
    Ok(())
}

pub(crate) fn validate_video_poll_request(
    request: &ServiceRequest<VideoPollPayload>,
) -> Result<(), ServiceError> {
    validate_service_request(request)?;
    validate_provider_secret_policy(&request.payload.api_key, request)?;
    if request
        .payload
        .project_id
        .is_some_and(|project_id| project_id <= 0)
        || request.payload.project_id != request.project_id
    {
        return Err(ServiceError {
            code: ServiceErrorCode::ConfigurationError,
            message: "video poll/status payload project_id 必须与 request envelope 一致且为正整数"
                .to_string(),
            retryable: Some(false),
        });
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
    validate_known_service_request_envelope(request, VIDEO_SERVICE_NAME)?;
    validate_egress_scope(
        VIDEO_SERVICE_NAME,
        EgressDataType::Video,
        &request.secret_policy.scope,
    )
    .map(|_| ())
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
    ServiceResponse {
        status: ServiceStatus::Error,
        trace_id: Some(request.trace_context.trace_id.clone()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: None,
        error: Some(error),
    }
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
    ServiceError {
        code: service_error_code(err),
        message: redact_sensitive_message(err.to_string()),
        retryable: Some(false),
    }
}
