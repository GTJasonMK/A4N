use a4n_core::error::AppError;
use a4n_service_protocol::{
    RAG_SERVICE_NAME, ServiceError, ServiceErrorCode, ServiceRequest, ServiceResponse,
    ServiceStatus, redact_sensitive_message, validate_known_service_request_envelope,
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
            error: Some(service_error(&err)),
        },
    }
}

pub(crate) fn validate_service_request<T>(request: &ServiceRequest<T>) -> Result<(), ServiceError> {
    validate_known_service_request_envelope(request, RAG_SERVICE_NAME)
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
        AppError::InvalidArgument(_) | AppError::ConfigError(_) => {
            ServiceErrorCode::ConfigurationError
        }
        AppError::VectorStoreError(_) => ServiceErrorCode::ProviderError,
        AppError::Cancelled => ServiceErrorCode::Cancelled,
        _ => ServiceErrorCode::Internal,
    }
}

pub(crate) fn service_error(err: &AppError) -> ServiceError {
    ServiceError {
        code: service_error_code(err),
        message: redact_sensitive_message(err.to_string()),
        retryable: Some(false),
    }
}
