use super::super::responses::unauthorized;
use crate::*;

pub(crate) async fn health(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
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

pub(crate) async fn capabilities(State(state): State<WorkerState>, headers: HeaderMap) -> Response {
    if !state.authorized(&headers) {
        return unauthorized();
    }
    Json(WorkerCapabilities {
        schema_version: WORKER_SCHEMA_VERSION,
        worker_id: state.worker_id.to_string(),
        capabilities: vec![
            WorkerCapability {
                name: SHOOTING_SCRIPT_CURRENT_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_UNIT_PROGRESS_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_EDIT_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_EDIT_PROGRESS_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_DELETE_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: SHOOTING_SCRIPT_PUBLISH_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
            WorkerCapability {
                name: VISUAL_PLAN_PERSISTENCE_CAPABILITY.into(),
                version: NOVEL_PLANNING_WORKER_CONTRACT_VERSION.into(),
                enabled: true,
            },
        ],
    })
    .into_response()
}
