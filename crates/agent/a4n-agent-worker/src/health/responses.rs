use a4n_error::AppError;
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerEnvelope, WorkerError, WorkerErrorCode, WorkerOperationRef,
    WorkerOperationStatus, WorkerResponse,
};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use serde_json::Value;

pub(super) fn app_error_failure(error: AppError) -> WorkerFailure {
    match error {
        AppError::InvalidArgument(message) => WorkerFailure::invalid_request(message),
        AppError::NotFound(message) => WorkerFailure::not_found(message),
        AppError::Forbidden(message) | AppError::Unauthorized(message) => {
            WorkerFailure::unauthorized(message)
        }
        AppError::InvalidState { current, operation } => {
            WorkerFailure::conflict(format!("{current}; operation={operation}"))
        }
        other => WorkerFailure::internal(other.to_string()),
    }
}

#[derive(Debug)]
pub(super) struct WorkerFailure {
    code: WorkerErrorCode,
    message: String,
    retryable: bool,
}

impl WorkerFailure {
    pub(super) fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::InvalidRequest,
            message: message.into(),
            retryable: false,
        }
    }

    pub(super) fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Unauthorized,
            message: message.into(),
            retryable: false,
        }
    }

    fn not_found(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::NotFound,
            message: message.into(),
            retryable: false,
        }
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Conflict,
            message: message.into(),
            retryable: false,
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            code: WorkerErrorCode::Internal,
            message: message.into(),
            retryable: false,
        }
    }
}

pub(super) fn completed<T: Serialize>(
    request: &WorkerEnvelope<Value>,
    payload: T,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Completed,
        payload: Some(payload),
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: None,
    }
}

pub(super) fn failed<T>(
    request: &WorkerEnvelope<Value>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: request.request_id.clone(),
        trace_id: request.trace_id.clone(),
        operation: WorkerOperationRef {
            operation_id: request.operation_id.clone(),
            task_id: None,
        },
        status: WorkerOperationStatus::Failed,
        payload: None,
        resource_refs: Vec::new(),
        committed_revision: None,
        provider_job_id: None,
        result_summary: None,
        pagination: None,
        error: Some(WorkerError {
            code: failure.code,
            message: failure.message,
            retryable: failure.retryable,
        }),
    }
}

pub(super) fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Agent worker authentication required",
    )
        .into_response()
}
