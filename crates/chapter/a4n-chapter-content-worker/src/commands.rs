use a4n_chapter_content_worker_contract::{
    ChapterInput, ImportChapterInput, ProjectInput, UpdateChapterContentInput,
};
use a4n_worker_contract::WorkerEnvelope;
use axum::{Json, extract::State, http::HeaderMap, response::Response};

use super::{WorkerState, errors::WorkerFailure, responses::unauthorized};

pub(super) async fn list(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.list(request)).await
}

pub(super) async fn analyses(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.analyses(request)).await
}

pub(super) async fn index(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ProjectInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.index(request)).await
}

pub(super) async fn summary(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.summary(request)).await
}

pub(super) async fn update(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<UpdateChapterContentInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| state.update(request)).await
}

pub(super) async fn import_chapter(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<ImportChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.import_chapter(request)
    })
    .await
}

async fn command_response<T, O, F, Fut>(
    state: &WorkerState,
    headers: &HeaderMap,
    request: WorkerEnvelope<T>,
    command: F,
) -> Response
where
    T: serde::Serialize + Clone,
    O: serde::Serialize,
    F: FnOnce(WorkerEnvelope<T>) -> Fut,
    Fut: std::future::Future<Output = Result<O, WorkerFailure>>,
{
    a4n_worker_runtime::worker_command_response(
        state.authorized(headers),
        request,
        command,
        unauthorized,
    )
    .await
}
