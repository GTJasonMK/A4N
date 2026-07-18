use a4n_core::error::AppError;
use a4n_service_protocol::{
    PROMPT_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    ServiceStatus, redact_sensitive_message, validate_known_service_request_envelope,
};

pub(crate) fn ok_response<T>(
    request: &ServiceRequest<impl Sized>,
    payload: T,
) -> ServiceResponse<T> {
    ServiceResponse {
        status: ServiceStatus::Ok,
        trace_id: Some(request.trace_context.trace_id.clone()),
        usage: None,
        artifact_refs: Vec::new(),
        payload: Some(payload),
        error: None,
    }
}

pub(crate) fn result_response<T>(
    request: &ServiceRequest<impl Sized>,
    result: Result<T, AppError>,
) -> ServiceResponse<T> {
    match result {
        Ok(payload) => ok_response(request, payload),
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

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request_envelope(request, PROMPT_SERVICE_NAME)
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

fn service_error_code(err: &AppError) -> ServiceErrorCode {
    match err {
        AppError::InvalidArgument(_) => ServiceErrorCode::ConfigurationError,
        AppError::ConfigError(_) => ServiceErrorCode::ConfigurationError,
        AppError::NotFound(_) => ServiceErrorCode::ConfigurationError,
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        _ => ServiceErrorCode::Internal,
    }
}
