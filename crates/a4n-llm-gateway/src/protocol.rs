use a4n_core::error::AppError;
use a4n_service_protocol::{
    LLM_SERVICE_NAME, SecretForwardingPolicy, ServiceError, ServiceErrorCode, ServiceRequest,
    ServiceResponse, ServiceStatus, redact_sensitive_message,
    validate_known_service_request_envelope, validate_provider_api_key_secret_policy,
};

pub(super) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request_envelope(request, LLM_SERVICE_NAME)
}

pub(super) fn validate_provider_secret_policy(
    api_key: Option<&str>,
    policy: &SecretForwardingPolicy,
) -> Result<(), ServiceError> {
    validate_provider_api_key_secret_policy(api_key, policy, LLM_SERVICE_NAME)
}

pub(super) fn result_response<T>(
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
            error: Some(service_error(err)),
        },
    }
}

pub(super) fn error_response<T>(
    request: &ServiceRequest<impl Sized>,
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
    ServiceError {
        code: service_error_code(&err),
        message: redact_sensitive_message(err.to_string()),
        retryable: Some(matches!(err, AppError::Timeout(_) | AppError::LlmError(_))),
    }
}
