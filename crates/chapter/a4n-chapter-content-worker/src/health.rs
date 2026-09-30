use super::{WorkerState, responses::unauthorized};
use a4n_chapter_content_worker_contract::{
    CHAPTER_CONTENT_CAPABILITY, CHAPTER_CONTENT_WORKER_CONTRACT_VERSION,
};
use a4n_worker_contract::{
    WORKER_SCHEMA_VERSION, WorkerCapabilities, WorkerCapability, WorkerHealth, WorkerHealthStatus,
};
use axum::{
    Json,
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response},
};
use sea_orm::{ConnectionTrait, Statement};

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
        capabilities: vec![WorkerCapability {
            name: CHAPTER_CONTENT_CAPABILITY.into(),
            version: CHAPTER_CONTENT_WORKER_CONTRACT_VERSION.into(),
            enabled: true,
        }],
    })
    .into_response()
}
