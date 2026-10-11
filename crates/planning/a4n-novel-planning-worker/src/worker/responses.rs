use crate::{WorkerEnvelope, WorkerFailure, WorkerResponse};
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
    (
        StatusCode::UNAUTHORIZED,
        "novel-planning worker authentication required",
    )
        .into_response()
}
