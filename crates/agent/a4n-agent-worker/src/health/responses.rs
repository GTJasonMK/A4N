use a4n_error::AppError;
pub(super) use a4n_worker_contract::WorkerFailure;
use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};
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

pub(super) fn completed<T: Serialize>(
    request: &WorkerEnvelope<Value>,
    payload: T,
) -> WorkerResponse<T> {
    WorkerResponse::completed_for(request, payload)
}

pub(super) fn failed<T>(
    request: &WorkerEnvelope<Value>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse::failed_for(request, failure.into())
}

pub(super) fn unauthorized() -> Response {
    (
        StatusCode::UNAUTHORIZED,
        "Agent worker authentication required",
    )
        .into_response()
}
