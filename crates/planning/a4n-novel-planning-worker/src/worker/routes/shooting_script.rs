use super::super::responses::{completed, failed, unauthorized};
use crate::*;

pub(crate) async fn current(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterShootingScriptSourceInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.current(request.clone()).await {
        Ok(payload) => completed(&request, OptionalWorkerPayload { value: payload }),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn unit_progress(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterShootingScriptSourceInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.unit_progress(request.clone()).await {
        Ok(payload) => completed(&request, OptionalWorkerPayload { value: payload }),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn edit(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EditChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.edit(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn edit_progress(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<EditChapterShootingScriptProgressInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.edit_progress(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn delete(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<DeleteChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.delete(request.clone()).await {
        Ok(()) => completed(&request, ()),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

pub(crate) async fn publish(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<PublishChapterShootingScriptInput>>,
) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let response = match state.publish(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}
