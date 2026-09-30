use super::*;

pub(super) async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    let status = match state.database.ping().await {
        Ok(()) => WorkerHealthStatus::Healthy,
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
                name: ADMIN_READ_CAPABILITY.into(),
                version: ADMIN_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: ADMIN_WRITE_CAPABILITY.into(),
                version: ADMIN_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}
