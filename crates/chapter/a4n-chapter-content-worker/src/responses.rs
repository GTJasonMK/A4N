use super::errors::WorkerFailure;
use a4n_worker_contract::{WorkerEnvelope, WorkerResponse};
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub(super) fn completed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    payload: T,
) -> WorkerResponse<T> {
    WorkerResponse::completed_for(request, payload)
}

pub(super) fn failed<T>(
    request: &WorkerEnvelope<impl serde::Serialize>,
    failure: WorkerFailure,
) -> WorkerResponse<T> {
    WorkerResponse::failed_for(request, failure.into())
}

pub(super) fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
