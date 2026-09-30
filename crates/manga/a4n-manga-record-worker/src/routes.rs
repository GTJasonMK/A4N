use a4n_manga_record_worker_contract::{
    MANGA_PIPELINE_CAPABILITY, MANGA_RECORD_CAPABILITY, MANGA_RECORD_WORKER_CONTRACT_VERSION,
    MangaChapterInput, MangaPipelineStatusesInput,
};
use a4n_worker_contract::{
    WORKER_CAPABILITIES_PATH, WORKER_HEALTH_PATH, WORKER_SCHEMA_VERSION, WorkerCapabilities,
    WorkerCapability, WorkerEnvelope, WorkerHealth, WorkerHealthStatus,
};
use axum::{
    Json,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use sea_orm::{ConnectionTrait, Statement};

use super::{
    WorkerState,
    protocol::{WorkerFailure, completed, failed},
};

pub(super) async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state
        .database
        .connection()
        .execute(Statement::from_string(
            sea_orm::DatabaseBackend::Postgres,
            "SELECT 1",
        ))
        .await
    {
        Ok(_) => WorkerHealthStatus::Healthy,
        Err(_) => WorkerHealthStatus::Degraded,
    };
    Json(WorkerHealth {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        status,
        version: env!("CARGO_PKG_VERSION").into(),
    })
    .into_response()
}

pub(super) async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: vec![
            WorkerCapability {
                name: MANGA_RECORD_CAPABILITY.into(),
                version: MANGA_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: MANGA_PIPELINE_CAPABILITY.into(),
                version: MANGA_RECORD_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}

pub(super) async fn list_page_prompts(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.list_page_prompts(request)
    })
    .await
}

pub(super) async fn pipeline_status(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaChapterInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.pipeline_status(request)
    })
    .await
}

pub(super) async fn pipeline_statuses(
    State(state): State<WorkerState>,
    headers: HeaderMap,
    Json(request): Json<WorkerEnvelope<MangaPipelineStatusesInput>>,
) -> Response {
    command_response(&state, &headers, request, |request| {
        state.pipeline_statuses(request)
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
    if !state.authorized(headers) {
        return unauthorized();
    }
    let response = match command(request.clone()).await {
        Ok(payload) => completed(&request, payload),
        Err(error) => failed(&request, error),
    };
    Json(response).into_response()
}

fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}

pub(super) fn router(state: WorkerState) -> axum::Router {
    axum::Router::new()
        .route(WORKER_HEALTH_PATH, axum::routing::get(health))
        .route(WORKER_CAPABILITIES_PATH, axum::routing::get(capabilities))
        .route(
            "/v1/commands/manga.record.list_page_prompts",
            axum::routing::post(list_page_prompts),
        )
        .route(
            "/v1/commands/manga.pipeline.status",
            axum::routing::post(pipeline_status),
        )
        .route(
            "/v1/commands/manga.pipeline.statuses",
            axum::routing::post(pipeline_statuses),
        )
        .with_state(state)
}
